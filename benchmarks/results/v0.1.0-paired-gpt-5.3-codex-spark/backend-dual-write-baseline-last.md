Implemented with a minimal code change.

- Updated [src/customer.py](/work/notyet-paired-2n89ih7z/backend-dual-write-baseline/src/customer.py:4) so `save_customer(...)` now stores `customer["email"]` as `email.lower()`, normalizing email to lowercase before persistence.

