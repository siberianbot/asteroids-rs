Legend:

* [+] fixed

- [+] Main thread hangs when dispatches window resize to rendering backend
    - Was fixed by introducing event for resized window
    - Dispatcher thread hangs now

- [+] Application does not crashes when worker thread panics
    - Event::WindowResized is handled without explicit swapchain recreation
    - Renderer is only one who recreates swapchain
    - R&D: why mutex with swapchain is locked?

- [+] Resize of window does not changes size of viewport
    - Surface capabilities were not updated after window resize
    - Removed clamp between min/max extent from capabilities