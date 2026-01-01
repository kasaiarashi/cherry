# Diff

Diff support is available natively in Cherry.

- Tree-sitter: [zed-industries/the-mikedavis/tree-sitter-diff](https://github.com/the-mikedavis/tree-sitter-diff)

## Configuration

Cherry will not attempt to format diff files and has [`remove_trailing_whitespace_on_save`](https://kriaa.in/cherry/docs/configuring-zed#remove-trailing-whitespace-on-save) and [`ensure-final-newline-on-save`](https://kriaa.in/cherry/docs/configuring-zed#ensure-final-newline-on-save) set to false.

Cherry will automatically recognize files with `patch` and `diff` extensions as Diff files. To recognize other extensions, add them to `file_types` in your Cherry settings.json:

```json [settings]
  "file_types": {
    "Diff": ["dif"]
  },
```
