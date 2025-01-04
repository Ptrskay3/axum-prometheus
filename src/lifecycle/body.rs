use super::{Callbacks, FailedAt, OnBodyChunk};
use futures_core::ready;
use http::HeaderValue;
use http_body::{Body, Frame};
use pin_project_lite::pin_project;
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use tower_http::classify::ClassifyEos;

pin_project! {
/// Response body for [`LifeCycle`].
pub struct ResponseBody<B, C, Callbacks, OnBodyChunk, CallbacksData> {
    #[pin]
    pub(super) inner: B,
    pub(super) parts: Option<(C, Callbacks)>,
    pub(super) callbacks_data: CallbacksData,
    pub(super) on_body_chunk: OnBodyChunk,
    pub(super) content_length: Option<HeaderValue>,
}
}

impl<B, C, CallbacksT, OnBodyChunkT, CallbacksData> Body
    for ResponseBody<B, C, CallbacksT, OnBodyChunkT, CallbacksData>
where
    B: Body,
    B::Error: fmt::Display + 'static,
    C: ClassifyEos,
    CallbacksT: Callbacks<C::FailureClass, Data = CallbacksData>,
    OnBodyChunkT: OnBodyChunk<B::Data, Data = CallbacksData>,
    CallbacksData: Clone,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();

        let body_size = this.inner.size_hint().exact().or_else(|| {
            this.content_length
                .as_ref()
                .and_then(|cl| cl.to_str().ok())
                .and_then(|cl| cl.parse().ok())
        });
        let result = ready!(this.inner.poll_frame(cx));

        match result {
            Some(Ok(frame)) => {
                let frame = match frame.into_data() {
                    Ok(chunk) => {
                        this.on_body_chunk
                            .call(&chunk, body_size, this.callbacks_data);
                        Frame::data(chunk)
                    }
                    Err(frame) => frame,
                };

                let frame = match frame.into_trailers() {
                    Ok(trailers) => {
                        if let Some((classify_eos, callbacks)) = this.parts.take() {
                            let classification = classify_eos.classify_eos(Some(&trailers));
                            callbacks.on_eos(
                                Some(&trailers),
                                classification,
                                this.callbacks_data.clone(),
                            );
                        }
                        Frame::trailers(trailers)
                    }
                    Err(frame) => frame,
                };

                Poll::Ready(Some(Ok(frame)))
            }
            Some(Err(err)) => {
                if let Some((classify_eos, callbacks)) = this.parts.take() {
                    let classification = classify_eos.classify_error(&err);
                    callbacks.on_failure(FailedAt::Body, classification, this.callbacks_data);
                }

                Poll::Ready(Some(Err(err)))
            }
            None => {
                if let Some((classify_eos, callbacks)) = this.parts.take() {
                    let classification = classify_eos.classify_eos(None);
                    callbacks.on_eos(None, classification, this.callbacks_data.clone());
                }
                Poll::Ready(None)
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
