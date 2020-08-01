use futures::{stream::Stream, task::Context};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::Poll;

mod json_array_stream;
mod json_depth_analyzer;

use json_array_stream::JsonArrayStream;
pub use json_array_stream::{stream_json_array, JsonStreamError};

pub struct ParsedStream<T, S, B>
where
    S: Stream<Item = B>,
    B: IntoIterator<Item = u8> + Sized,
{
    stream: JsonArrayStream<S, B>,
    _t: PhantomData<T>,
}

impl<'de, S, B> JsonArrayStream<S, B>
where
    S: Stream<Item = B>,
    B: IntoIterator<Item = u8> + Sized,
{
    pub fn parsed<T>(self) -> ParsedStream<T, S, B>
    where
        T: serde::de::Deserialize<'de>,
    {
        return ParsedStream {
            stream: self,
            _t: PhantomData::<T>,
        };
    }
}

impl<T, S, B> Stream for ParsedStream<T, S, B>
where
    S: Stream<Item = B>,
    B: IntoIterator<Item = u8> + Sized,
    T: for<'de> serde::de::Deserialize<'de>,
{
    type Item = Result<T, JsonStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut this.stream) }.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(opt) => Poll::Ready(opt.map(|res| {
                res.and_then(|buffer| {
                    serde_json::from_slice(&buffer).map_err(|err| JsonStreamError::from(err))
                })
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::TryStreamExt;

    #[tokio::test]
    async fn owned_array() {
        let json = "[-12,11.1,0]";
        let stream = futures::stream::once(async { json.bytes() });
        let parsed: Result<Vec<_>, _> = stream_json_array(stream)
            .parsed::<f64>()
            .try_collect()
            .await;

        assert_eq!(parsed.unwrap(), vec![-12., 11.1, 0.]);
    }
}
