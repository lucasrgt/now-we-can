Implemented the smallest possible change in `src/PendingPayment.tsx`:

- Updated the pending-payment message to use a live, screen-reader-announced status region by adding `role="status"` and `aria-live="polite"`.

This makes the message announce itself as live status (`Payment pending`).