Legend:

* [+] done
* [~] in progress
* [?] not sure

- Asteroids game made in Rust

    - Rendering

        - Draw a window
            - Add winit
            - Show a window

        - Vulkan
            - R&D: suitable rendering library for Rust?
                - vulkano?
                - ash-rs?
            - VK backend initialization
            - VK renderer initialization

        - Shaders
            - Entities rendering
            - UI rendering

        - Utils for various purposes
            - Texture atlas
            
        - UI rendering

            - Fonts
                - Building texture atlas
                - ...

            - ...

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

    - Utils
        - [+] Worker for parallel execution (i.e. game state update)

    - Physics
        - R&D

    - [?] Configuration
        - Window configuration
        - Renderer configuration
        - Controls

    - [?] Logs

    - ...
