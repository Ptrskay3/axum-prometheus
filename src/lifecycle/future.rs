use axum_core::response::Response;
use futures_core::ready;
use http_body::Body;
use pin_project::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_http::classify::{ClassifiedResponse, ClassifyResponse};

use crate::MetricsData;

use super::{body::ResponseBody, Callbacks, FailedAt};

#[pin_project]
pub struct ResponseFuture<F, C, Callbacks, OnBodyChunk, CallbackData> {
    #[pin]
    pub(super) inner: F,
    pub(super) classifier: Option<C>,
    pub(super) callbacks: Option<Callbacks>,
    pub(super) on_body_chunk: Option<OnBodyChunk>,
    pub(super) callbacks_data: Option<CallbackData>,
}

pub trait OnBodyChunk<B> {
    type Data;

    #[inline]
    fn on_body_chunk(&mut self, _body: &B, _data: &mut Self::Data) {
        println!("on body chunk with data");
    }
}

impl<B> OnBodyChunk<B> for ()
where
    B: bytes::Buf,
{
    type Data = Option<MetricsData>;

    #[inline]
    fn on_body_chunk(&mut self, body: &B, data: &mut Self::Data) {
        if let Some(metrics_data) = data {
            metrics_data.body_size += body.remaining();
            let labels = &[
                ("method", metrics_data.method.to_owned()),
                ("endpoint", metrics_data.endpoint.clone()),
            ];
            metrics::histogram!("axum_http_body_size", metrics_data.body_size as f64, labels);
        }
    }
}

impl<F, C, CallbacksData, ResBody, E, CallbacksT, OnBodyChunkT> Future
    for ResponseFuture<F, C, CallbacksT, OnBodyChunkT, CallbacksData>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    ResBody: Body,
    C: ClassifyResponse,
    CallbacksT: Callbacks<C::FailureClass, Data = CallbacksData>,
    E: std::fmt::Display + 'static,
    OnBodyChunkT: OnBodyChunk<ResBody::Data, Data = CallbacksData>,
    CallbacksData: Clone,
{
    type Output = Result<
        Response<ResponseBody<ResBody, C::ClassifyEos, CallbacksT, OnBodyChunkT, CallbacksT::Data>>,
        E,
    >;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = ready!(this.inner.poll(cx));

        let classifier = this
            .classifier
            .take()
            .expect("polled future after completion");
        let mut callbacks = this
            .callbacks
            .take()
            .expect("polled future after completion");
        let mut callbacks_data = this
            .callbacks_data
            .take()
            .expect("polled future after completion");

        let on_body_chunk = this.on_body_chunk.take().unwrap();

        match result {
            Ok(res) => {
                let classification = classifier.classify_response(&res);

                match classification {
                    ClassifiedResponse::Ready(classification) => {
                        callbacks.on_response(
                            &res,
                            ClassifiedResponse::Ready(classification),
                            &mut callbacks_data,
                        );
                        let res = res.map(|body| ResponseBody {
                            inner: body,
                            parts: None,
                            on_body_chunk,
                            callbacks_data: callbacks_data.clone(),
                        });
                        Poll::Ready(Ok(res))
                    }
                    ClassifiedResponse::RequiresEos(classify_eos) => {
                        callbacks.on_response(
                            &res,
                            ClassifiedResponse::RequiresEos(()),
                            &mut callbacks_data,
                        );
                        let res = res.map(|body| ResponseBody {
                            inner: body,
                            callbacks_data: callbacks_data.clone(),
                            on_body_chunk,
                            parts: Some((classify_eos, callbacks)),
                        });
                        Poll::Ready(Ok(res))
                    }
                }
            }
            Err(err) => {
                let classification = classifier.classify_error(&err);
                callbacks.on_failure(FailedAt::Response, classification, callbacks_data);
                Poll::Ready(Err(err))
            }
        }
    }
}
