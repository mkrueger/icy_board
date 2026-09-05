# FTN/FidoNet correctness review — 2026-09-05

Scope: Binkp session and transfer handling, FTN packets and bundles, inbound
tossing, outbound scanning, point addressing, TIC file echoes and file requests.
Existing unrelated upload-processing changes were left untouched.

## Fixed findings

| Severity | Component | Failure and correction |
| --- | --- | --- |
| High | Auto-add | Network-provided echo tags could escape `new_areas` through absolute paths or `../`. Invalid path-like tags are now rejected; their packet is retained in the bad-packet directory. |
| High | Auto-add/JAM | Dotted tags such as `RU.LINUX` and `RU.WINDOWS` shared a base because JAM replaces the path extension. Generated names now escape punctuation, including the escape character itself, to avoid collisions. Existing explicitly configured area paths are not migrated. |
| High | TIC security | With secure processing enabled, omitting `From` or supplying an unparsable address bypassed the configured-source check. A configured source is now required even in these cases. Non-secure mode remains permissive. |
| High | TIC replacement | `Replaces` files were deleted before the replacement was moved and indexed. Superseded files are now removed only after those steps succeed. This is not a full filesystem/database transaction. |
| High | Binkp handshake | An early `M_OK` bypassed the remote-address check and password exchange. It is now rejected until that exchange has occurred. This is a protocol-order check, not additional cryptographic server authentication. |
| High | Binkp retries | `M_GET` for a fully transmitted but unacknowledged file was ignored, including after `M_EOB`. Such requests now queue a retransmission without losing another active file; EOF, cancellation and repeated-request cases are covered. |
| High | Binkp receive | A frame exceeding the advertised remaining file size was written and positively acknowledged. It now fails before writing the offending frame, without publishing or acknowledging the corrupt file. Malformed offsets are also rejected; `M_FILE -1` negotiation remains supported. |
| Medium | Point addressing | Points emitted their boss's 2D address as their own PATH/SEEN-BY entry, and pass-through treated sibling points as the same source. Points are now excluded from generated 2D entries and source comparisons use the full address. A boss's SEEN-BY entry no longer excludes its points. |
| Medium | File requests | A 20-byte Unicode request filename could panic during byte-indexed parsing. ASCII is checked before slicing. |
| Medium | File requests | Merging requests without a final newline concatenated two requested names. A separator is now inserted. |

## Remaining limitations

- **Not a complete hub tosser:** incoming echomail for a locally carried or
  auto-added area is imported but not forwarded to other subscribers. Pass-through
  only handles uncarried areas; outbound scanning deliberately skips imported mail.
- **Inbound routing is packet-level:** a packet addressed to this system is
  imported even when an enclosed netmail message names another destination.
  Message-level transit routing is not implemented. Consequently, readdressing
  packets to a next hop requires a next-hop tosser that can perform that routing.
- Publication of bundles, scan state and message-base updates is not one atomic
  transaction. Crash recovery and simultaneous mailer processes need a separate
  durability/concurrency review; passing the tests below does not certify these.
- No live interoperability session with binkd or another independent mailer was
  performed. Transfer tests use deterministic peers and local connections.

## Validation

- Added 22 regression tests, including a check that deleted local echomail is
  already excluded correctly (no production change needed for that suspicion).
- Before the fixes, five new tests reproduced path traversal, dotted-tag
  collisions, both point-addressing errors and the premature `M_OK` acceptance.
- FTN module tests: **135 passed**.
- Binkp module tests: **45 passed**.
- Broader engine/network/mailer library and binary tests: **1,588 passed,
  5 ignored**. This is not the complete workspace/integration suite.
- Formatting checks for the changed Rust files and `git diff --check` passed.