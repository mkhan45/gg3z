Making an explicit state relation is verbose but makes explicit
the fact that we have to solve the whole state at a time.

```
state : type.
state.mk : int -> int -> state
state.next : int -> int -> state
step : state -> input -> state -> type.

step/jump : step (state.mk Y DY) keys (state.next Y DY')
    <- Y = 0
    <- plus DY 10 DY'
    <- pressed_jump keys.

step/fall : step (state.mk Y DY) keys (state.next Y' DY')
    <- Y > 0
    <- plus Y DY Y'
    <- plus DY G DY'.

% query step (state.mk ...) ... S'
% ... produces next state

% query (state.mk Y 10)
```
