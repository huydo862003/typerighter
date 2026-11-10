---
_label: "Icons"
description: "Reference for Typedown's built-in icon module. Use Lucide icons with the _icon field to add page icons in your vault."
_icon: icon.image
---

Icons give your pages visual identity in the sidebar and page header, making it easier to scan a large vault at a glance. Set `_icon` in the frontmatter to any value from the built-in `icon` module, which wraps a curated set of [Lucide](https://lucide.dev) icons.

```yaml
_icon: icon.rocket
```

## Usage

```yaml
---
_type: Article
_icon: icon.rocket
title: "Launch plan"
---
```

The icon appears in the sidebar navigation and the page header.

## Syntax

Icons use dot access on the `icon` module. Only `icon.X` syntax is valid. Raw strings like `_icon: "rocket"` are type errors.

```yaml
_icon: icon.rocket      # correct
_icon: icon.lightbulb   # correct
_icon: "rocket"         # type error
```

## Available icons

### Documents

[[ icon-row {td="document" lucide="file-text"} ]]
[[ icon-row {td="page" lucide="file"} ]]
[[ icon-row {td="notebook" lucide="notebook"} ]]
[[ icon-row {td="article" lucide="newspaper"} ]]
[[ icon-row {td="scroll" lucide="scroll"} ]]
[[ icon-row {td="book" lucide="book-open"} ]]
[[ icon-row {td="book_marked" lucide="book-marked"} ]]
[[ icon-row {td="clipboard" lucide="clipboard"} ]]
[[ icon-row {td="note" lucide="sticky-note"} ]]
[[ icon-row {td="file_code" lucide="file-code"} ]]

### Folders and organization

[[ icon-row {td="folder" lucide="folder"} ]]
[[ icon-row {td="archive" lucide="archive"} ]]
[[ icon-row {td="container" lucide="box"} ]]
[[ icon-row {td="package" lucide="package"} ]]
[[ icon-row {td="briefcase" lucide="briefcase"} ]]
[[ icon-row {td="inbox" lucide="inbox"} ]]
[[ icon-row {td="layers" lucide="layers"} ]]
[[ icon-row {td="library" lucide="library"} ]]

### Communication

[[ icon-row {td="mail" lucide="mail"} ]]
[[ icon-row {td="message" lucide="message-circle"} ]]
[[ icon-row {td="megaphone" lucide="megaphone"} ]]
[[ icon-row {td="bell" lucide="bell"} ]]
[[ icon-row {td="phone" lucide="phone"} ]]
[[ icon-row {td="radio" lucide="radio"} ]]
[[ icon-row {td="send" lucide="send"} ]]

### People

[[ icon-row {td="user" lucide="user"} ]]
[[ icon-row {td="users" lucide="users"} ]]
[[ icon-row {td="contact" lucide="contact"} ]]
[[ icon-row {td="hand" lucide="hand"} ]]

### Nature

[[ icon-row {td="leaf" lucide="leaf"} ]]
[[ icon-row {td="flower" lucide="flower-2"} ]]
[[ icon-row {td="tree" lucide="tree-pine"} ]]
[[ icon-row {td="sun" lucide="sun"} ]]
[[ icon-row {td="moon" lucide="moon"} ]]
[[ icon-row {td="cloud" lucide="cloud"} ]]
[[ icon-row {td="mountain" lucide="mountain"} ]]
[[ icon-row {td="waves" lucide="waves"} ]]
[[ icon-row {td="snowflake" lucide="snowflake"} ]]
[[ icon-row {td="flame" lucide="flame"} ]]
[[ icon-row {td="droplet" lucide="droplet"} ]]

### Science and tech

[[ icon-row {td="flask" lucide="flask-conical"} ]]
[[ icon-row {td="atom" lucide="atom"} ]]
[[ icon-row {td="cpu" lucide="cpu"} ]]
[[ icon-row {td="code" lucide="code"} ]]
[[ icon-row {td="terminal" lucide="terminal"} ]]
[[ icon-row {td="database" lucide="database"} ]]
[[ icon-row {td="server" lucide="server"} ]]
[[ icon-row {td="bug" lucide="bug"} ]]
[[ icon-row {td="circuit" lucide="circuit-board"} ]]
[[ icon-row {td="binary" lucide="binary"} ]]

### Creative

[[ icon-row {td="palette" lucide="palette"} ]]
[[ icon-row {td="pen" lucide="pen"} ]]
[[ icon-row {td="brush" lucide="paintbrush"} ]]
[[ icon-row {td="camera" lucide="camera"} ]]
[[ icon-row {td="music" lucide="music"} ]]
[[ icon-row {td="film" lucide="film"} ]]
[[ icon-row {td="image" lucide="image"} ]]
[[ icon-row {td="mic" lucide="mic"} ]]
[[ icon-row {td="headphones" lucide="headphones"} ]]

### Status and emotion

[[ icon-row {td="heart" lucide="heart"} ]]
[[ icon-row {td="star" lucide="star"} ]]
[[ icon-row {td="flag" lucide="flag"} ]]
[[ icon-row {td="trophy" lucide="trophy"} ]]
[[ icon-row {td="target" lucide="target"} ]]
[[ icon-row {td="bookmark" lucide="bookmark"} ]]
[[ icon-row {td="thumbs_up" lucide="thumbs-up"} ]]
[[ icon-row {td="check" lucide="circle-check"} ]]
[[ icon-row {td="alert" lucide="triangle-alert"} ]]
[[ icon-row {td="info" lucide="info"} ]]
[[ icon-row {td="sparkles" lucide="sparkles"} ]]
[[ icon-row {td="crown" lucide="crown"} ]]

### Shapes

[[ icon-row {td="circle" lucide="circle"} ]]
[[ icon-row {td="square" lucide="square"} ]]
[[ icon-row {td="diamond" lucide="diamond"} ]]
[[ icon-row {td="hexagon" lucide="hexagon"} ]]
[[ icon-row {td="triangle" lucide="triangle"} ]]
[[ icon-row {td="zap" lucide="zap"} ]]
[[ icon-row {td="hash" lucide="hash"} ]]
[[ icon-row {td="at_sign" lucide="at-sign"} ]]
[[ icon-row {td="infinity" lucide="infinity"} ]]

### Navigation and travel

[[ icon-row {td="compass" lucide="compass"} ]]
[[ icon-row {td="map" lucide="map"} ]]
[[ icon-row {td="globe" lucide="globe"} ]]
[[ icon-row {td="home" lucide="house"} ]]
[[ icon-row {td="rocket" lucide="rocket"} ]]
[[ icon-row {td="plane" lucide="plane"} ]]
[[ icon-row {td="car" lucide="car"} ]]
[[ icon-row {td="ship" lucide="ship"} ]]
[[ icon-row {td="signpost" lucide="signpost"} ]]

### Time

[[ icon-row {td="clock" lucide="clock"} ]]
[[ icon-row {td="calendar" lucide="calendar"} ]]
[[ icon-row {td="timer" lucide="timer"} ]]
[[ icon-row {td="hourglass" lucide="hourglass"} ]]

### Tools

[[ icon-row {td="wrench" lucide="wrench"} ]]
[[ icon-row {td="hammer" lucide="hammer"} ]]
[[ icon-row {td="key" lucide="key"} ]]
[[ icon-row {td="lock" lucide="lock"} ]]
[[ icon-row {td="shield" lucide="shield"} ]]
[[ icon-row {td="link" lucide="link"} ]]
[[ icon-row {td="magnet" lucide="magnet"} ]]
[[ icon-row {td="scissors" lucide="scissors"} ]]
[[ icon-row {td="lightbulb" lucide="lightbulb"} ]]
[[ icon-row {td="glasses" lucide="glasses"} ]]

### Data and charts

[[ icon-row {td="chart" lucide="chart-bar"} ]]
[[ icon-row {td="pie_chart" lucide="chart-pie"} ]]
[[ icon-row {td="trending" lucide="trending-up"} ]]
[[ icon-row {td="table" lucide="table"} ]]
[[ icon-row {td="kanban" lucide="kanban"} ]]

### Education

[[ icon-row {td="graduation" lucide="graduation-cap"} ]]
[[ icon-row {td="school" lucide="school"} ]]
[[ icon-row {td="pencil" lucide="pencil"} ]]

### Food

[[ icon-row {td="coffee" lucide="coffee"} ]]
[[ icon-row {td="pizza" lucide="pizza"} ]]
[[ icon-row {td="apple" lucide="apple"} ]]

### Misc

[[ icon-row {td="gift" lucide="gift"} ]]
[[ icon-row {td="puzzle" lucide="puzzle"} ]]
[[ icon-row {td="gamepad" lucide="gamepad-2"} ]]
[[ icon-row {td="ticket" lucide="ticket"} ]]
[[ icon-row {td="tag" lucide="tag"} ]]
[[ icon-row {td="pin" lucide="pin"} ]]
[[ icon-row {td="trash" lucide="trash-2"} ]]
[[ icon-row {td="wand" lucide="wand-sparkles"} ]]
[[ icon-row {td="eye" lucide="eye"} ]]
[[ icon-row {td="search" lucide="search"} ]]
[[ icon-row {td="settings" lucide="settings"} ]]
