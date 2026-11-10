# Independent native-client verification

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Static production caller and guard inspection; no Windows native execution.

## CL-005 — VIP presence shortcut precludes its own window creation

**CONFIRMED P2.** In an ordinary initially closed state, the actual `PlayLink::vip_presence_input` sets `mailbox.vip_presence.open = true` before invoking the Windows `view::open_window` (`apps/client/src/features/0258-vip-presence/input.rs:4-11`). The view directly reexports `native::open_window`. That function locks the same mailbox and immediately returns when `open` is true (`native.rs:17-23`). It therefore never starts the window thread or reaches CreateWindowExW for this opening request.

Reachability is verified in `apps/client/src/windows_shell/input.rs:125-152`, which calls the handler in the native input chain. The UI shortcut is a nonrepeating pressed Control+Shift keycode25 without Alt/Super (`crates/client-ui/src/features/0258-vip-presence/input.rs`). The preceding ordinary VIP handler explicitly excludes Shift (`apps/client/src/features/0043-vip/input.rs`), so it does not consume this chord. The native window's own flag serves as a duplicate-window guard; the earlier write defeats first creation rather than supplying a required preparation step.

The demonstrated effect is failure to open this particular presence window by its wired shortcut. Other VIP windows, server presence projections, authority and persisted data are outside the finding. The inspected test only checks panel text for a selected capability; it does not exercise native window creation. No runtime test pass or deployed incidence is claimed.

Recommendation: keep ownership of the opening flag at the actual native creation boundary, while preserving its duplicate-window guard and rollback on thread/window creation failure. This audit contains no source fix.
