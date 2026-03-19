# Day Planner Design Specifications

This document outlines the design and requirements for the Day Planner JIRA time-reporting TUI application.

## Technical Requirements

- **Language**: Rust
- **TUI Framework**: Ratatui
- **Terminal Backend**: Crossterm
- **Data Persistence**: JSON-based storage located in the user's home directory (`~/.jira-time-reporter.json`).
- **Date Management**: Chrono for handling naive dates and time durations.
- **Input Parsing**: Regex parsing for extracting task names and time spent (supports hours and minutes).
- **Unique Identification**: Uuid for tracking individual task records.

## Features

- **Day Timeline**: Visual representation of the workday from 8:00 AM to 4:00 PM.
- **Task Management**: Simple string-based entry for tasks (e.g., "WMI-12345 DB Update") with time spent in "3h 25m" format.
- **Automatic Gap Calculation**: Dynamically computes and displays unused time between reported tasks within the 8 AM - 4 PM window.
- **Date Browsing**: Support for navigating through past and future days to view or add records.
- **Persistent Storage**: Every change is immediately synchronized with the underlying JSON file.
- **Date-Specific Records**: Tasks are organized and saved for the specific date they were reported on.

## UI

- **TUI Interface**: A terminal-based user interface using a block-based layout.
- **Vertical Timeline**: Situated on the left side, showing 1-hour increments with spacing for visual alignment.
- **Task Blocks**: Rectangular boxes signifying reported tasks, placed proportionately next to the timeline according to their duration.
- **Input Area**: A dedicated box at the bottom for entering new task records.
- **Header**: Displays the currently viewed date in the format: "Wed 18 Mar 2026".
- **Footer**: Shows keyboard shortcuts and controls (similar to the Nano editor's style).
- **Navigation**: Left and Right arrow keys used to cycle through different dates.
