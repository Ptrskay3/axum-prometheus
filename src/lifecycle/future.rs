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

use super::{body::ResponseBody, Callbacks, FailedAt, OnBodyChunk, OnExactBodySize};

#[pin_project]
pub struct ResponseFuture<F, C, Callbacks, OnBodyChunk, OnExactBodySize, CallbackData> {
    #[pin]
    pub(super) inner: F,
    pub(super) classifier: Option<C>,
    pub(super) callbacks: Option<Callbacks>,
    pub(super) on_body_chunk: Option<OnBodyChunk>,
    pub(super) callbacks_data: Option<CallbackData>,
    pub(super) on_exact_body_size: Option<OnExactBodySize>,
}

impl<F, C, CallbacksData, ResBody, E, CallbacksT, OnBodyChunkT, OnExactBodySizeT> Future
    for ResponseFuture<F, C, CallbacksT, OnBodyChunkT, OnExactBodySizeT, CallbacksData>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
    ResBody: Body,
    C: ClassifyResponse,
    CallbacksT: Callbacks<C::FailureClass, Data = CallbacksData>,
    E: std::fmt::Display + 'static,
    OnBodyChunkT: OnBodyChunk<ResBody::Data, Data = CallbacksData>,
    OnExactBodySizeT: OnExactBodySize<Data = CallbacksData>,
    CallbacksData: Clone,
{
    type Output = Result<
        Response<
            ResponseBody<
                ResBody,
                C::ClassifyEos,
                CallbacksT,
                OnBodyChunkT,
                OnExactBodySizeT,
                CallbacksT::Data,
            >,
        >,
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
        let on_body_chunk = this
            .on_body_chunk
            .take()
            .expect("polled future after completion");
        let on_exact_body_size = this
            .on_exact_body_size
            .take()
            .expect("polled future after completion");

        match result {
            Ok(res) => {
                let content_length = res.headers().get(http::header::CONTENT_LENGTH).cloned();
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
                            on_exact_body_size,
                            content_length,
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
                            on_exact_body_size,
                            content_length,
                        });
                        Poll::Ready(Ok(res))
                    }
                }
            }
            Err(err) => {
                let classification = classifier.classify_error(&err);
                callbacks.on_failure(FailedAt::Response, classification, &mut callbacks_data);
                Poll::Ready(Err(err))
            }
        }
    }
}
