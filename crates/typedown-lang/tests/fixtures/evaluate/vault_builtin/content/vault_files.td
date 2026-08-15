---
all: vault.files()
articles: vault.files_where({ schema: Article })
total: vault.count()
article_count: vault.count_where({ schema: Article })
note_count: vault.count_where({ schema: Note })
blog_count: vault.count_where({ path: "blog" })
blog_articles: vault.count_where({ schema: Article, path: "blog" })
---
