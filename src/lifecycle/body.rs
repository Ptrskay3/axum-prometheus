use super::{Callbacks, FailedAt, OnBodyChunk, OnExactBodySize};
use futures_core::ready;
use http_body::Body;
use pin_project::pin_project;
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use tower_http::classify::ClassifyEos;

/// Response body for [`LifeCycle`].
#[pin_project]
pub struct ResponseBody<B, C, Callbacks, OnBodyChunk, OnExactBodySize, CallbacksData> {
    #[pin]
    pub(super) inner: B,
    pub(super) parts: Option<(C, Callbacks)>,
    pub(super) callbacks_data: CallbacksData,
    pub(super) on_body_chunk: OnBodyChunk,
    pub(super) on_exact_body_size: OnExactBodySize,
}

impl<B, C, CallbacksT, OnBodyChunkT, OnExactBodySizeT, CallbacksData> Body
    for ResponseBody<B, C, CallbacksT, OnBodyChunkT, OnExactBodySizeT, CallbacksData>
where
    B: Body,
    B::Error: fmt::Display + 'static,
    C: ClassifyEos,
    CallbacksT: Callbacks<C::FailureClass, Data = CallbacksData>,
    OnBodyChunkT: OnBodyChunk<B::Data, Data = CallbacksData>,
    OnExactBodySizeT: OnExactBodySize<Data = CallbacksData>,
    CallbacksData: Clone,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.project();

        let body_size = this.inner.size_hint().exact();
        let result = if let Some(result) = ready!(this.inner.poll_data(cx)) {
            result
        } else {
            return Poll::Ready(None);
        };

        if let Some(exact_size) = body_size {
            this.on_exact_body_size
                .call(exact_size, this.callbacks_data);
        }

        match result {
            Ok(chunk) => {
                this.on_body_chunk.call(&chunk, this.callbacks_data, body_size);

                Poll::Ready(Some(Ok(chunk)))
            }
            Err(err) => {
                if let Some((classify_eos, callbacks)) = this.parts.take() {
                    let classification = classify_eos.classify_error(&err);
                    callbacks.on_failure(FailedAt::Body, classification, this.callbacks_data);
                }

                Poll::Ready(Some(Err(err)))
            }
        }
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<http::HeaderMap>, Self::Error>> {
        let this = self.project();

        let result = ready!(this.inner.poll_trailers(cx));

        match result {
            Ok(trailers) => {
                if let Some((classify_eos, callbacks)) = this.parts.take() {
                    let trailers = trailers.as_ref();
                    let classification = classify_eos.classify_eos(trailers);
                    callbacks.on_eos(trailers, classification, this.callbacks_data.clone());
                }

                Poll::Ready(Ok(trailers))
            }
            Err(err) => {
                if let Some((classify_eos, callbacks)) = this.parts.take() {
                    let classification = classify_eos.classify_error(&err);
                    callbacks.on_failure(FailedAt::Trailers, classification, this.callbacks_data);
                }

                Poll::Ready(Err(err))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}
