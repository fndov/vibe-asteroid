# Vibe-asteroid

A CLI-based arcade game written in Rust, inspired by classic asteroid-dodging games, with a focus on geometric backend and ASCII frontend rendering.

## Concept

Players control a spaceship navigating through a field of asteroids within the terminal. The game objects (ship, asteroids) have precise geometric shapes defined in the backend. The frontend's responsibility is to represent these shapes using appropriate ASCII characters on the console, effectively "highlighting" the characters at their coordinates.

## Interface & Rendering

- The game runs entirely within the command-line interface.
- All game elements are rendered using ASCII characters.
- **Geometric Backend:** Ship is a diamond shape, asteroids are "bumpy" shapes. These shapes are defined by a set of relative coordinates.
- **ASCII Frontend:** The display layer determines which ASCII characters to use to represent the geometric shapes at their calculated positions.
- The display is designed to be fast and responsive.

## Controls

- Controls are displayed in a small box in the bottom-left corner of the terminal.

## Features

- **Player Ship:** Represented by a diamond shape.
- **Asteroids:** Represented by "bumpy" shapes.
- **Movement:** Player controls ship movement.
- **Collision Detection:** Based on overlapping geometric shapes.
- **Scoring:** A scoring system is implemented.
- **Game Over:** Clear game over condition and display.
- **Difficulty Scaling:** Increasing difficulty over time.
- **Debug Mode:** Allows for simulated rendering to an in-memory buffer (logged to file) and simulated input injection for internal testing and debugging. Configurable resolution for debug mode.

## Technology

- Rust for backend logic and rendering.
- `crossterm` for terminal manipulation (raw mode, cursor control).
- `rand` for random generation.
