Hey there was an interesting piece of syntax I have never encountered before on https://tokio.rs/docs/futures/spawning/

There is an enum defined as follows.

```rust
    #[derive(Eq, PartialEq)]
    enum Item {
        Value(usize),
        Tick,
        Done,
    }
 ```

 The `Stream::map` method was used as shown below.

```rust
    let items = rx.map(Item::Value)
    // --snip--
```

I am confused how an enum cna be passed in here.
Is this equivalent to

```rust
    let items = rx.map(|len| Item::Value(len))
```

Confusingly enough, this syntax also works,
```
    let items = rx.map(|len| Item::Value);
```