# TODO

## Runtime

- [x] Tracing migration -- replace all `println!`/`eprintln!` with `tracing` macros
- [x] Code cleanup -- collapse repeated lock patterns into helpers, clean up unused import warnings
- [ ] Stats/diagnostics query -- add a `Command::Stats` or similar for querying compute load and cycle usage from the Handle
- [ ] Graceful clean exit -- handle firmware returning from `_start` cleanly (currently breaks the loop but doesn't signal the Handle properly)
- [x] RuntimeError cleanup -- remove unused `WriteError::ValueOverflow` variant
- [x] Firmware visibility -- `engine` and `module` on `Image` are already private

## Testing

- [ ] Shared mock MCU -- duplicated between `tests/blink.rs` and `examples/blink_runner.rs`, extract to a `test-support` feature or module
- [ ] Reset test -- verify cold/warm reset works via Handle
- [ ] Error reporting test -- verify Handle receives the actual error after firmware trap
- [ ] Lifecycle test -- verify Off -> Running -> Halted -> Reset -> Running flow
- [ ] Busy-loop test -- verify busy-loop firmware gates on cycle budget and resumes on tick

## Future

- [ ] Watchdog peripheral -- device that triggers reset when firmware doesn't pet it
- [ ] Timer peripheral -- device that fires interrupts at configured intervals
- [ ] Default ISR handler -- firmware-configurable catch-all for missing `__isr_N` exports
- [ ] Debug logging redesign -- research how real firmware logging works (semihosting, RTT, etc.)
