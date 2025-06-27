# TODO - Vibe-asteroid (Geometric Rendering)

## Core Concepts

- **Backend:** Defines the game's logic, physics, and the precise geometric shapes of objects (e.g., ship, asteroids) as 2D coordinate sets. These shapes are abstract and not tied to specific ASCII characters at this layer.
- **Frontend:** Responsible for rendering the backend's geometric data onto the terminal. It takes the calculated positions and shapes from the backend and determines which ASCII characters to use to represent them on the console. It does not contain pre-defined ASCII art for complex shapes; instead, it places individual characters at the coordinates provided by the backend's geometric definition.

## Tasks

- [x] Core Game Loop: Implement a basic game loop (update, render).
- [x] Terminal Setup: Configure terminal for raw mode, hide cursor, and handle initial clear.
- [x] Debug Mode: Implement simulated rendering to `ScreenBuffer` (which will directly store characters based on backend geometry) and simulated input.
- [x] Game Grid: Implement a `GameGrid` struct to hold the entire game state as characters, acting as the canvas for the frontend.
- [x] Ship Geometry: Define the diamond shape for the ship as a set of relative coordinates. Implement its drawing onto the `GameGrid` by placing characters at these coordinates.
- [x] Asteroid Geometry: Define bumpy shapes for asteroids as sets of relative coordinates. Implement their drawing onto the `GameGrid` by placing characters at these coordinates.
- [x] Player Ship Movement: Implement movement for the ship, updating its geometric position.
- [x] Asteroid Generation & Movement: Generate and move asteroids, updating their geometric positions.
- [x] Collision Detection: Implement collision detection based on overlapping geometric shapes (or their rendered characters on the `GameGrid`).
- [x] Score: Implement a scoring system.
- [x] Game Over: Implement game over condition and display.
- [x] Controls Display: Show controls in a bottom-left box.
- [x] Speed/Difficulty: Implement increasing difficulty over time.
- [x] Title Screen: Display a title screen with ASCII art (this can be pre-defined as it's not a game object).
- [x] Cross-platform compatibility (Linux first).