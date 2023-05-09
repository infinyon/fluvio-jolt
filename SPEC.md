# Specification for this implementation of jolt

## Expression Grammar

Grammar for right hand side and left hand side expressions.

`x+` means one or more x, `x*` means zero or more x, `x?` means zero or one x.

```
Lhs: AtExpr |
        DollarSignExpr |
        SquareExpr |
        AmpExpr |
        Pipes;

Rhs: ('[' IndexOp? ']' | RhsEntry*) RhsPart*;
RhsPart: '[' IndexOp? ']' | '.' RhsEntry*;
RhsEntry: AmpExpr |
            AtExpr |
            Key;
IndexOp: AmpExpr
            | Number
            | AtExpr;

AtExpr: '@' AtTuple?;
AtTuple: '(' Index ',' RHS ')' | '(' Rhs ')';
DollarSignExpr: '$' NumTuple?;
NumTuple: '(' Index ',' Index ')' | '(' Index ')';
SquareExpr: '#' Key;
AmpExpr: '&' NumTuple?;
Pipes: Stars ( '|' Stars )*;
Stars: Key ( '*' Key )*;

Key: <any non-empty string of characters>
Number: '1-9' '0-9'+;
```

## Syntactic sugar

- `&(x)` is equal to `&(x, 0)`.
- `$(x)` is equal to `$(x, 0)`.
- `@(Rhs)` is equal to `@(0, Rhs)`,

## Escape sequences

`@`, `$`, `#`, `&`, `[`, `]`, `|`, `.`, `,`, `(`, `)`, `*`, `\` can be escaped using a `\`.

## Infallible/fallible lhs expressions and execution order

`@`, `$` and `#` expressions are considered infallible, and the rest is considered fallible.

First the infallible expressions are executed and they are executed once.

Keys are executed in the order they are specified in the spec.

So if the spec is:
```json
{
    "$": "a.b.c",
    "hello": "world",
    "@": "q.w.e",
    "&": "b"
}
```
The `$` will be executed first and then the `@` will be executed.

Then for each key in the input:
- First `hello` will be executed.
- If `hello` didn't match, the `&` will be executed.

## Behavior

When accessing a value from the input:
- Execution errors if key is not found when accessing an object.
- Execution errors if an index is out of range when indexing into an array.

When outputting a value to the output:
- If a key is not found in the object, it is initialized to an empty object.
- If an index is out of range when accessing an array, the array is extended using null values.
- If some value already exists in the target, the execution errors out.
- The rhs expression has to specify that the output is an array in order to push to an array like `my.path[]`.
Otherwise the execution will error.
