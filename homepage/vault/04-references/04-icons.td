The `icon` module provides a curated set of page icons based on [Lucide](https://lucide.dev). Use them with the `_icon` built-in field.

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

| Name | Lucide |
| ---- | ------ |
| `icon.document` | file-text |
| `icon.page` | file |
| `icon.notebook` | notebook |
| `icon.article` | newspaper |
| `icon.scroll` | scroll |
| `icon.book` | book-open |
| `icon.book_marked` | book-marked |
| `icon.clipboard` | clipboard |
| `icon.note` | sticky-note |
| `icon.file_code` | file-code |

### Folders and organization

| Name | Lucide |
| ---- | ------ |
| `icon.folder` | folder |
| `icon.archive` | archive |
| `icon.container` | box |
| `icon.package` | package |
| `icon.briefcase` | briefcase |
| `icon.inbox` | inbox |
| `icon.layers` | layers |
| `icon.library` | library |

### Communication

| Name | Lucide |
| ---- | ------ |
| `icon.mail` | mail |
| `icon.message` | message-circle |
| `icon.megaphone` | megaphone |
| `icon.bell` | bell |
| `icon.phone` | phone |
| `icon.radio` | radio |
| `icon.send` | send |

### People

| Name | Lucide |
| ---- | ------ |
| `icon.user` | user |
| `icon.users` | users |
| `icon.contact` | contact |
| `icon.hand` | hand |

### Nature

| Name | Lucide |
| ---- | ------ |
| `icon.leaf` | leaf |
| `icon.flower` | flower-2 |
| `icon.tree` | tree-pine |
| `icon.sun` | sun |
| `icon.moon` | moon |
| `icon.cloud` | cloud |
| `icon.mountain` | mountain |
| `icon.waves` | waves |
| `icon.snowflake` | snowflake |
| `icon.flame` | flame |
| `icon.droplet` | droplet |

### Science and tech

| Name | Lucide |
| ---- | ------ |
| `icon.flask` | flask-conical |
| `icon.atom` | atom |
| `icon.cpu` | cpu |
| `icon.code` | code |
| `icon.terminal` | terminal |
| `icon.database` | database |
| `icon.server` | server |
| `icon.bug` | bug |
| `icon.circuit` | circuit-board |
| `icon.binary` | binary |

### Creative

| Name | Lucide |
| ---- | ------ |
| `icon.palette` | palette |
| `icon.pen` | pen |
| `icon.brush` | paintbrush |
| `icon.camera` | camera |
| `icon.music` | music |
| `icon.film` | film |
| `icon.image` | image |
| `icon.mic` | mic |
| `icon.headphones` | headphones |

### Status and emotion

| Name | Lucide |
| ---- | ------ |
| `icon.heart` | heart |
| `icon.star` | star |
| `icon.flag` | flag |
| `icon.trophy` | trophy |
| `icon.target` | target |
| `icon.bookmark` | bookmark |
| `icon.thumbs_up` | thumbs-up |
| `icon.check` | circle-check |
| `icon.alert` | triangle-alert |
| `icon.info` | info |
| `icon.sparkles` | sparkles |
| `icon.crown` | crown |

### Shapes

| Name | Lucide |
| ---- | ------ |
| `icon.circle` | circle |
| `icon.square` | square |
| `icon.diamond` | diamond |
| `icon.hexagon` | hexagon |
| `icon.triangle` | triangle |
| `icon.zap` | zap |
| `icon.hash` | hash |
| `icon.at_sign` | at-sign |
| `icon.infinity` | infinity |

### Navigation and travel

| Name | Lucide |
| ---- | ------ |
| `icon.compass` | compass |
| `icon.map` | map |
| `icon.globe` | globe |
| `icon.home` | house |
| `icon.rocket` | rocket |
| `icon.plane` | plane |
| `icon.car` | car |
| `icon.ship` | ship |
| `icon.signpost` | signpost |

### Time

| Name | Lucide |
| ---- | ------ |
| `icon.clock` | clock |
| `icon.calendar` | calendar |
| `icon.timer` | timer |
| `icon.hourglass` | hourglass |

### Tools

| Name | Lucide |
| ---- | ------ |
| `icon.wrench` | wrench |
| `icon.hammer` | hammer |
| `icon.key` | key |
| `icon.lock` | lock |
| `icon.shield` | shield |
| `icon.link` | link |
| `icon.magnet` | magnet |
| `icon.scissors` | scissors |
| `icon.lightbulb` | lightbulb |
| `icon.glasses` | glasses |

### Data and charts

| Name | Lucide |
| ---- | ------ |
| `icon.chart` | chart-bar |
| `icon.pie_chart` | chart-pie |
| `icon.trending` | trending-up |
| `icon.table` | table |
| `icon.kanban` | kanban |

### Education

| Name | Lucide |
| ---- | ------ |
| `icon.graduation` | graduation-cap |
| `icon.school` | school |
| `icon.pencil` | pencil |

### Food

| Name | Lucide |
| ---- | ------ |
| `icon.coffee` | coffee |
| `icon.pizza` | pizza |
| `icon.apple` | apple |

### Misc

| Name | Lucide |
| ---- | ------ |
| `icon.gift` | gift |
| `icon.puzzle` | puzzle |
| `icon.gamepad` | gamepad-2 |
| `icon.ticket` | ticket |
| `icon.tag` | tag |
| `icon.pin` | pin |
| `icon.trash` | trash-2 |
| `icon.wand` | wand-sparkles |
| `icon.eye` | eye |
| `icon.search` | search |
| `icon.settings` | settings |
