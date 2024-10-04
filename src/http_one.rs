use std::future::Future;
use std::io::{BufWriter, Read};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

pub struct HttpOneService<S: HttpOneHandler+Service<INNER_S>, INNER_S> {
    inner: Option<S>,
    _a: PhantomData<INNER_S>
}

pub trait HttpOneHandler {

}

/// Used to resolve request and write response
pub struct HttpOneHandlerFuture<R: Read, H: HttpOneHandler> {
    handler: H,
    input_stream: R,
}

impl <R:Read, H: HttpOneHandler> Future for HttpOneHandlerFuture<R, H> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!()
    }
}

/// Used to send response for call
pub struct HttpOneServiceFuture<R: Read, H: HttpOneHandler> {
    inner: Option<H>,
    input_stream: Option<R>,
}

impl <R: Read, H: HttpOneHandler> Future for HttpOneServiceFuture<R, H> {
    type Output = Result<(flume::Receiver<u8>, HttpOneHandlerFuture<R, H>), ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (sx, rx) = flume::bounded(100);
        Poll::Ready(Ok((rx, HttpOneHandlerFuture{ handler: self.inner.take().unwrap(), input_stream: self.input_stream.take().unwrap() })))
    }
}

impl <R: Read, H: HttpOneHandler + Service<INNER_S>, INNER_S> Service<R> for HttpOneService<H, INNER_S> {
    type Response = (flume::Receiver<u8>, HttpOneHandlerFuture<R, H>);
    type Error = ();
    type Future = HttpOneServiceFuture<R, H>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.inner {
            None => panic!("Inner handler is None"),
            Some(i) => i.poll_ready(cx),
        }
    }

    // Provided an input stream, generate response stream
    fn call(&mut self, req: R) -> Self::Future {
        HttpOneServiceFuture {
            inner: self.inner.take(),
            input_stream: Some(req),
        }
    }
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;
    use tower::Service;
    use crate::http_one::{HttpOneHandler, HttpOneService};

    pub struct FakeHandler {

    }

    impl HttpOneHandler for FakeHandler {

    }

    #[tokio::test]
    async fn test_get() {
        let mut service = HttpOneService{ inner: Some(FakeHandler{} ), _a: PhantomData };
        let Ok((receiver, request_future)) = service.call("GET / HTTP/1.1\r\n\r\n".as_bytes()).await;
        assert!(receiver.is_empty());
        let a = request_future.await;
        let data =receiver.drain().collect::<Vec<u8>>();
        assert_eq!(data, b"HTTP/1.1 200 OK\r\n\r\nHello, World!");
    }
}