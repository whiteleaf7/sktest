(1/2) の段階の状態です

## 使い方

```
cargo run -q -- <file> <schema>
```

- `<file>`: 検証する sysctl.conf 形式のファイル
- `<schema>`: キーと型を書いた TOML のスキーマファイル

スキーマに合っていれば、parse した結果を表示して終了コード 0 で終わります。合っていなければ、1 行に 1 つずつエラーを stderr に出して、終了コード 1 で終わります。エラーの先頭には、直すべきファイルのパスが付きます。

## スキーマの書き方

```toml
endpoint = "string"
debug = "bool"
retry = "integer"
log.file = "string"
log.level = "string?"
```

- 型は `string`、`bool`、`integer` の 3 つです
  - `string`: 空文字も含めて何でも通します
  - `bool`: 小文字の `true` か `false` だけ通します
  - `integer`: 64 ビットの符号付き整数（Rust の `i64`）だけ通します
- 型名の後ろに `?` を付けたキーは任意です。それ以外のキーは必須です
- ネストしたキーは `log.file` か `[log]` のセクションで書きます
- スキーマにないキーが対象のファイルにあると、エラーになります

## コマンド例

コマンドは p2 のディレクトリで実行します。例のファイルは [examples](examples) にあります。

### スキーマに合っているファイル

```
cargo run -q -- examples/ok.conf examples/schema.toml
```

parse した結果の map を表示して、終了コード 0 で終わります。map のキーの並びは実行するたびに変わります。

### スキーマに合っていないファイル

```
cargo run -q -- examples/ng.conf examples/schema.toml
```

見つかった違反をすべて、キーの順に並べて出します。終了コードは 1 です。

```
examples/ng.conf: db.host: expected string, got table
examples/ng.conf: db.port: expected integer, got "abc"
examples/ng.conf: debug: expected bool (true or false), got "True"
examples/ng.conf: log: expected table, got string "/var/log/console.log"
examples/ng.conf: retry: expected integer, got "1_000"
examples/ng.conf: retyr: unknown key
```

- `db.host`: スキーマでは値なのに、`db.host.name` と書いたため、テーブルになっています
- `db.port`: integer として読めない値です
- `debug`: bool は小文字の `true` / `false` だけなので、`True` は通りません
- `log`: スキーマではテーブル（`log.file`、`log.level`）なのに、値を直接書いています
- `retry`: `1_000` は Rust の `i64` として読めないので通りません
- `retyr`: スキーマにないキーです（`retry` の打ち間違い）

### 壊れたスキーマ

```
cargo run -q -- examples/ok.conf examples/broken.toml
```

スキーマに知らない型名（`intger`）が書かれています。スキーマのパスと、行と列を付けたエラーを出して、終了コード 1 で終わります。

```
examples/broken.toml: line 3, column 9: unknown type "intger" (expected string, bool or integer, optionally followed by '?')
```

### 書き方が壊れたファイル

```
cargo run -q -- examples/bad-syntax.conf examples/schema.toml
```

`=` のない行があるため、スキーマで検証する前の parse の段階で失敗します。終了コードは 1 です。

```
examples/bad-syntax.conf: line 1: missing '='
```

### 引数が足りない

```
cargo run -q -- examples/ok.conf
```

引数は `<file>` と `<schema>` の 2 つとも必須です。使い方を出して、終了コード 1 で終わります。

```
usage: p1 <file> <schema>
```

### ファイルがない

```
cargo run -q -- examples/none.conf examples/schema.toml
```

ファイルを読めなかったことを、パスを付けて出します。終了コードは 1 です。

```
examples/none.conf: No such file or directory (os error 2)
```

### 終了コードを見る

```
cargo run -q -- examples/ng.conf examples/schema.toml; echo "exit: $?"
```

最後に `exit: 1` と表示されます。スキーマに合っているファイルなら `exit: 0` です。

## テストと検査

```
cargo test
cargo clippy --all-targets
cargo fmt --check
```
