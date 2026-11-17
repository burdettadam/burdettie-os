# Isolated native block-token removal observations

The existing Foundation reference fails before the Burdettie comparison. Its
foreign-center plain removal runs before its foreign-center named removal on
the same token, so the latter cannot characterize an independent operation.

This source-only diagnostic adds thirteen isolated native cases. Each creates
fresh owner/foreign centers, sender identities and a retained block token. A
foreign post cannot deliver, wrong name/sender posts cannot deliver for a
filtered registration, and an owning-center positive post must deliver exactly
once before the removal operation under observation. The observed post-removal
count (zero or one) is reported, not assigned an expected compatibility result.
Final owner cleanup must stop delivery, and objects are released only afterward.

The matrix includes plain removal through the owner, a foreign center and the
default center; matching, mismatching-name, mismatching-sender and nil-filter
named removal through owner/foreign centers; and specific-filter removal of a
wildcard registration through owner/foreign centers. Every named case receives
a new token, never one already removed by another case.

The existing native integration test compiles and runs this observer before its
unchanged 31-check compatibility gate. The original failure remains required;
this diagnostic cannot turn it into a pass. Compiler/runtime deadlines and log
bounds are reused unchanged; no workflow is modified. Its stdout is retained in
the same failing test's output, independently of the original report.

These are native host observations only. Do not infer guest qualification,
asynchronous queues, block-copy ownership, or Darwin release-independent API
behavior. Apple documents name/sender removal filters; observed implementation
behavior must be recorded separately from that documented contract. Results
remain pending until the current-head native CI run executes the probe.
