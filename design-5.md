```
// other relations/rules

State =
    Tick : int
    X : float
    Y : float
    Dx : float
    Dy : float
where
    State { 0, 1.0, 0.0, 0.0, 0.0 }

    State { T+1, X', Y', Dx', Dy' } :-
        State { T, X, Y, Dx, Dy } ∧
        add(X, Dx, X') ∧
        when Y = 0:
            add(Y, Dy, Y')
            Dy' = 0
        or when Y = 0 ∧ key(inp, space):
            add(Y, Dy, Y')
            add(Dy, 5.0, Dy')
```

To use tables updating through state, we could pass through a Tick column
to every relation? Need primary keys or something to prevent duplication?

What about different syntax for asserting a new ground fact vs satisfying a relation?
