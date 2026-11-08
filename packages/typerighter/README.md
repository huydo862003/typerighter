# typerighter

See [README.npm.md](README.npm.md) for the package documentation, or visit the [homepage](https://huydo862003.github.io/typerighter/).

## Technical Debts

- Currently, fake index page for directories are handled a bit adhocly, causing them to be easily missed... However, I expect the core logic of the plugin to changes slowly so this can be accepted for now.

## Learnt Lessons

### Flash of Uninited State in SSR

For SSR pages, full initial renders are prerendered into HTML and they are sent to the client & hydrated later.

However, the full initial render can lack some information, like local storage on client side, so the initially rendered UI can be different from what the client should see on first render.

I encountered this in this project: ThemeToggle getting flipped right after hydration, Sidebar getting its width shifted after hydration...

These cause certain jitters and layout shifts that are annoying.

So my approach is:
- Generate blocking head script that reads from local storage and sets CSS variables.
- In the component, I use the CSS variables so SSR won't inline it and on first render, the CSS takes effect immediately.

-> This completely eliminates the problem, at the cost of some hoops.

See: https://github.com/huydo862003/typerighter/commit/5139adc8cf2b87f6ef15eb11a9c1c3a7fc88e7e5

Next step: Can we abstract this?
