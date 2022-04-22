use futures::Stream;

struct TakeBytes<S> {
    inner: S,
    seen: usize,
    limit: usize,
}
