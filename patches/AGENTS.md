# patches/AGENTS.md

## このディレクトリの役割

`patches/` は、`upstream/main` にまだ取り込まれていない downstream
変更を、`main` に適用する順序付きパッチ列として管理する場所である。

このディレクトリでは、次の運用を正本とする。

- `patches/*.patch` が downstream 変更の正本である。
- `main` がパッチの適用先であり、`patches/*.patch` をすべて番号順に適用したソースを保持する。
- `upstream/main` は `main` を更新するときの基底であり、パッチを適用していない状態をビルド対象にしない。
- パッチ適用後に作られるソースの変更は、`main` の変更として扱う。
- `patches/README.md` は、このファイルと矛盾しない手順だけを記載する。

## mainへの適用

`patches/` 配下の `.patch` は、例外なく `main` に対して適用する。適用済み
ソースを別worktreeだけに置いたままにせず、`main` のソースが全パッチ適用済みの
状態になるように保つ。

## 適用

パッチを `main` のworktreeへ適用するときは `git apply` を使う。
`git am` はコミットを作るため、通常の適用には使わない。

適用前に、`main` に未コミットの作業がないことを確認する。パッチはファイル名の
番号順にすべて適用し、途中のパッチを省略しない。

```bash
git switch main
git status --short

for patch in patches/*.patch; do
  [ -e "$patch" ] || break
  git apply "$patch" || exit 1
done
```

`git apply --3way` を使う場合はindexも変更されることがある。適用に失敗したら
パッチを読み飛ばさず、原因を解消してからパッチ列を最初から適用し直す。

## upstream更新

`main` に適用済みのソースを残したまま `upstream/main` を更新しない。次のどちらかを使う。

1. `main` のパッチを番号の逆順に取り消し、基底を更新してから再適用する。
2. 新しい `main` を作り直し、パッチを再適用する。

パッチの適用に失敗した場合は、パッチを無理に読み飛ばさない。原因を確認し、
一時queue worktreeで競合を解消してからパッチファイルを再生成し、`main` に全パッチを
再適用する。

## パッチ列の編集と再生成

パッチの内容を変更するときは、一時queue worktreeを `upstream/main` から作る。
既存パッチの適用と競合解消には `git am --3way` を使ってよい。一時queue
worktree内のコミットは、パッチ列を編集・検証・再生成するための作業用である。

検証後は一時queue worktreeのコミット列から `git format-patch` でパッチを再生成し、
`patches/` のファイルを置き換える。パッチ列の順序、変更内容、メタデータを確認して
から、`main` に全パッチを再適用する。

## コミット前の確認

コミット前には、依頼されたパッチファイル、運用文書、`main` に適用したソースが
意図した対象になっていることを確認する。

```bash
git diff --cached --check -- patches/README.md patches/AGENTS.md
git diff --cached --name-only
git status --short
```

適用済みのソースファイルを、パッチ適用の依頼に反してindexから外してはならない。
履歴を書き換える場合は、作業前に旧 `main` をbackup refで保持し、別worktreeで
新しい履歴を検証する。
