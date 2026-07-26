Implemented in `/work/notyet-paired-2n89ih7z/backend-dual-write-notyet/src/customer.py` with the smallest complete change.

- `save_customer` now stores normalized email via `email.lower()`.
- Removed the `legacy_name` dual-write.

Not-Yet obligations handled:
- Ran `notyet wake --event mobile-v1-retired` (cue became due).
- Resolved deferment `benchmark-backend-dual-write` with evidence of the `customer.py` update.
- Ran `notyet check --event mobile-v1-retired` and got: `No deferments are due.`

Ready to commit.