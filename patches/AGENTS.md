# patches/AGENTS.md

## このディレクトリの役割

`patches/` は、`upstream/main` にまだ取り込まれていない downstream
変更を、適用順のあるパッチ列として管理する場所である。

このディレクトリでは、次の運用を正本とする。

- `patches/*.patch` が downstream 変更の正本である。
- `upstream/main` がパッチを適用する基底である。
- パッチ適用後に作られるソースの変更は、適用用worktreeの未コミット変更として扱う。
- 適用済みソースの変更を `main` のコミットに含めない。
- `patches/README.md` は、このファイルと矛盾しない手順だけを記載する。

## 適用

パッチを実際のビルド用worktreeへ適用するときは `git apply` を使う。
`git am` はコミットを作るため、通常の適用には使わない。

適用前に、対象worktreeに未コミットの作業がないことを確認する。パッチは
ファイル名の番号順に適用する。

```bash
git status --short

for patch in patches/*.patch; do
  [ -e "$patch" ] || break
  git apply "$patch" || exit 1
done
```

`git apply --3way` を使う場合はindexも変更されることがある。適用済みソースを
コミットしない場合でも、パッチファイルだけをコミット対象にする前に、indexの
状態を確認する。

## upstream更新

適用済みのソースを残したまま `upstream/main` を更新しない。次のどちらかを使う。

1. 適用用worktreeでパッチを番号の逆順に取り消し、基底を更新してから再適用する。
2. 適用用worktreeを新しい `main` から作り直し、パッチを再適用する。

パッチの適用に失敗した場合は、パッチを無理に読み飛ばさない。原因を確認し、
一時queue worktreeで競合を解消してからパッチファイルを再生成する。

## パッチ列の編集と再生成

パッチの内容を変更するときは、一時queue worktreeを `upstream/main` から作る。
既存パッチの適用と競合解消には `git am --3way` を使ってよい。一時queue
worktree内のコミットは、パッチ列を編集・検証・再生成するための作業用であり、
正本の `main` に残さない。

検証後は一時queue worktreeのコミット列から `git format-patch` でパッチを再生成し、
`patches/` のファイルを置き換える。パッチ列の順序、変更内容、メタデータを確認して
から、パッチファイルと運用文書だけをコミットする。

## コミット前の確認

コミット対象は `patches/*.patch`、`patches/README.md`、このファイルに限定する。

```bash
git diff --cached --check -- patches/README.md patches/AGENTS.md
git diff --cached --name-only
git status --short
```

適用済みのソースファイルがindexに入っている場合は、パッチファイルをコミット
する前にindexから外す。履歴を書き換える場合は、作業前に旧 `main` をbackup refで
保持し、現在の `main` を直接編集せず、別worktreeで新しい履歴を検証する。
