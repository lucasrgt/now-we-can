Implemented.

Updated the list response to include the total number of orders in a new `total_count` field.

- File changed: [src/orders.py](/work/notyet-paired-2n89ih7z/roadmap-pagination-baseline/src/orders.py)
- `list_orders(db)` now:
  - calls `db.fetch_orders(limit=20)` as before
  - uses `db.count_orders()` if available (method)
  - falls back to `len(items)` if not available
  - returns `{"items": items, "total_count": total_count}`

No other changes made.