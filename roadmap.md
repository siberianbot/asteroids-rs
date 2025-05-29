Legend:

* [+] done
* [~] in progress

- Asteroids game made in Rust

    - Rendering

        - Draw a window
            - add winit

        - Vulkan
            - R&D: suitable rendering library for Rust?
                - vulkano?
                - ash-rs?

        - ...

    - Game logic

        - [~] Game commands
            - Player input
                - Forward acceleration
                - Backward acceleration
                - Incline (left/right)
                - Fire
            - [~] Exit
            - ...

        - [~] Game events
            - [~] Entity created
            - [~] Entity destroyed
            - [~] Collision detected
            - ...

        - Entities
            - Spacecraft entity
            - Asteroid entity
            - Camera entity

        - Game loop
            - Commands dispatching
            - Entities updated
            - Event dispatching

    - Physics
        - R&D

    - ...
