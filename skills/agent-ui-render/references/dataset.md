# Agent UI Dataset Contract

This is the home file for dataset rules in compact Agent UI input version 1.
`references/ui-input.md` defines the whole payload; this file defines how shared
tabular data is represented safely and compactly.

## Boundary

A dataset is data, not a UI component.

- Put reusable rows under top-level `d`.
- Views, metrics, and markdown must not embed duplicate row data.
- Views reference datasets and columns by index.
- Preserve source facts. Do not invent missing rows, units, labels, or meaning.
- Use alerts (`a`) or summary (`s`) for uncertainty and omissions.

## Compact shapes

A compact dataset is exactly one of these tuple shapes:

```ts
type DatasetTuple =
  | [id: string, columns: ColumnTuple[], rows: Primitive[][]]
  | [id: string, "cols", columns: ColumnTuple[], columnData: Primitive[][]]
  | [id: string, "ref", reference: string];
```

Use row-major rows by default:

```json
{
  "d": [
    [
      "sales",
      [["month", "s"], ["revenue", "cur", "USD"], ["growth", "pct"]],
      [["Jan", 120000, 0.05], ["Feb", 135000, 0.125]]
    ]
  ]
}
```

Use column-major only when it is clearly more compact for large, homogeneous
data:

```json
{
  "d": [
    [
      "sales",
      "cols",
      [["month", "s"], ["revenue", "cur", "USD"]],
      [["Jan", "Feb", "Mar"], [120000, 135000, 150000]]
    ]
  ]
}
```

## Column tuples

Column tuples are:

- `[key, type]`
- `[key, type, unit]`
- `[key, type, unit, label]`

Rules:

- `key` should be stable `lower_snake_case`.
- Keys must be unique within a dataset.
- Omit `label` when it can be derived from the key.
- Include `unit` for currency and measured numbers when known.
- Do not guess units. If unknown, omit the unit and disclose uncertainty.

Type codes:

- `s`: string
- `n`: number
- `cur`: currency number
- `pct`: percent as a ratio, for example `0.125` means `12.5%`
- `d`: date string
- `dt`: datetime string
- `b`: boolean
- `dict:<id>`: dictionary-coded string

## Row rules

Rows are arrays of primitive cells only:

```ts
type Primitive = string | number | boolean | null;
```

Rules:

- Every row length must equal the column count.
- Use `null` for known-missing cells.
- Do not shorten ragged rows.
- Do not use objects, arrays, HTML, CSS, JS, or component/action names as cells.
- Keep numeric chart measures numeric-compatible: `n`, `cur`, or `pct`.

## External refs

Use `[id, "ref", reference]` when the data already exists in a tool result,
database, upload, query result, or server-side cache and the host UI can resolve
it.

```json
{
  "version": 1,
  "d": [["sales", "ref", "query_result_01"]],
  "a": [["i", "Sales rows are resolved by the host UI."]],
  "v": [["r", 0]]
}
```

Standalone bundled HTML cannot resolve arbitrary refs. It renders a visible
placeholder/alert and may skip chart views that need missing column metadata.
For a standalone preview, prefer materialized rows.

## Dictionary encoding

Use `dict` only when repeated strings are long enough to justify indirection.
Dictionary values normalize back to strings.

```json
{
  "version": 1,
  "dict": {
    "status": ["confirmed", "supporting", "blocked"]
  },
  "d": [
    [
      "evidence",
      [["source", "s"], ["status", "dict:status"]],
      [["Deploy log", 0], ["Trace sample", 1], ["Database check", 2]]
    ]
  ]
}
```

## View compatibility

Use one shared dataset for multiple views:

```json
{
  "version": 1,
  "d": [
    [
      "sales",
      [["month", "s"], ["revenue", "cur", "USD"]],
      [["Jan", 120000], ["Feb", 135000], ["Mar", 150000]]
    ]
  ],
  "v": [["t", 0, 0, [1]], ["r", 0]]
}
```

Compatibility rules:

- `r` only needs a dataset index.
- `t`, `b`, `p`, and `d` need an x/dimension column index.
- Numeric chart measures must reference numeric-compatible columns.
- `s` needs numeric x plus at least one distinct numeric y measure.
- If a requested chart is invalid, use `r` and add an alert.

## Anti-patterns

Avoid object-array rows because keys repeat and row shape is harder to verify:

```json
[
  { "month": "Jan", "revenue": 120000 },
  { "month": "Feb", "revenue": 135000 }
]
```

Avoid embedding chart data in a view:

```json
{
  "type": "chart",
  "rows": [["Jan", 120000]]
}
```

Avoid markdown tables for records or evidence. Use datasets plus `r` views.
Avoid using string columns as numeric measures. Use a table and alert instead.
