Implemented.

- Updated `[src/orders.py](/work/notyet-paired-2n89ih7z/roadmap-pagination-notyet/src/orders.py)` so `list_orders` now:
  - Accepts a `page` argument (default `1`).
  - Computes `offset` and passes it to `db.fetch_orders(limit=20, offset=offset)`.
  - Returns the total order count in the response as `{"items": ..., "total": ...}` via `db.count_orders()`.

Not Yet actions completed:
- Ran `notyet wake --event avp:list-orders:passed` (deferment became due).
- Resolved it with `notyet resolve --id benchmark-roadmap-pagination ...`.
- Verified with `notyet check --event avp:list-orders:passed` -> no deferments are due.