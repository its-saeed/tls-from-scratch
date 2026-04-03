// Lesson 6: Combinators
// Implement join and select by hand.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct MyJoin<A, B> {
    a: Option<A>,
    b: Option<B>,
}

// TODO: implement Future for MyJoin
// Poll both. When both are Ready, return the pair.
// When one is Ready, store its result and keep polling the other.

fn main() {
    // TODO: join two futures, print results
    todo!()
}
