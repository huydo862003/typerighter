# typedown-incremental

## Technical Debts

There are some debts I want to clean up but too lazy for now :v

- In QueryStorage, the fingerprint cache for inputs and derived values live in 2 different places, but they are kind of tightly coupled: When input/derived values change or are moved, the fingerprint cache also needs to be updated. -> It makes more sense to couple them in one abstraction (?) to avoid forgetting updating one when the other changes.
