Legend:

* [+] fixed

- [+] Main thread hangs when dispatches window resize to rendering backend
    - Was fixed by introducing event for resized window
    - [ ] Dispatcher thread hangs now

- [ ] Application does not crashes when worker thread panics
