Begin Facts:
    StateVar Health
    StateVar Score
    eq(Health, 100)
    eq(Score, 0)
End Facts

Begin Global:
End Global

Begin Stage Update:
Begin State Constraints:
    int_sub(Health, 5, next(Health))
    int_add(Score, 10, next(Score))
End State Constraints
End Stage Update
