```
table enemy(id: int, x : float, y : float, hp : float)@t
struct player(x : float, y : float, dx : float, dy : float)@t

player(0.0, 0.0, 0.0, 0.0) @ 0

enemy(0, 0.0, 10.0, 10.0) @ 0
enemy(1, 5.0, 10.0, 10.0) @ 0

player(X, Y, Dx, Dy) @ t ∧ add(X, Dx, X') ∧ add(Y, Dy, Y')
-----------------------------------------------------------
player(X', Y', Dx, Dy) @ (t+1)
```
How do stages work?
