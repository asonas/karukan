# upstream ベースの downstream パッチ運用

このディレクトリは、`togatoga/karukan` の `upstream/main` にまだ取り込まれていない変更を、番号付きパッチ列として管理するための場所である。

パッチファイルが downstream 変更の正本である。`main` には upstream の履歴と `patches/` のパッチ・運用文書だけをコミットし、パッチを適用したソースコードは作業用 worktree に置く。詳細な規約は [`patches/AGENTS.md`](AGENTS.md) を参照する。

## 3つの状態

| 状態 | 用途 | コミットするもの |
| --- | --- | --- |
| `main` | upstream の更新とパッチ列を管理する | `patches/*.patch` と運用文書だけ |
| 適用済み worktree | ビルド、テスト、普段の開発に使う | 適用後のソースはコミットしない |
| 一時 queue worktree | パッチの編集、競合解消、再生成に使う | `git am` 用の一時コミット。作業後に破棄する |

## リモートの役割

| リモート | リポジトリ | 用途 |
| --- | --- | --- |
| `origin` | `asonas/karukan` | パッチ列と運用文書を含む自分用 `main` の push 先 |
| `upstream` | `togatoga/karukan` | 最新の upstream を fetch する先 |

`upstream` へ変更を pushしない。push 前には URL とブランチを確認する。履歴を書き換えた後の push が必要な場合も、対象ブランチを確認してから `--force-with-lease` を使う。

初回だけ、リモートを確認する。

~~~bash
git remote -v
git remote get-url origin
git remote get-url upstream
~~~

想定する URL と異なる場合は、次のように設定する。

~~~bash
git remote set-url origin git@github.com:asonas/karukan.git
git remote set-url upstream ssh://git@github.com/togatoga/karukan.git
~~~

## パッチのメタデータと順序

各パッチは、元になった upstream の PR と、自分用に取り込む理由を追跡できる状態にする。パッチの元になる一時コミットの本文に、次の項目を含める。

~~~text
Upstream-PR: https://github.com/togatoga/karukan/pull/123
Upstream-Commit: abcdef1234567890
Downstream-Reason: macOS で常用するために候補ウィンドウの挙動を変更する
Tested-On: macOS 15.x, arm64
~~~

`upstream` の PR をそのまま適用せず変更を加えた場合は、何を変更したかを `Downstream-Reason` に書く。パッチファイル名は適用順が明確になるようにする。

~~~text
0001-<短い説明>.patch
0002-<短い説明>.patch
~~~

適用順はファイル名の番号で定義する。シェルの glob の順序に依存せず、次のように番号順で適用する。

~~~bash
for patch in patches/*.patch; do
  [ -e "$patch" ] || break
  git apply "$patch" || exit 1
done
~~~

## 現在のローカル変更と upstream PR の対応

現在のローカル変更は、次の upstream PR / commit を元にしている。PR の commit をそのまま適用していない場合もあるため、個別の一時コミットには `Downstream-Reason` を残す。

| ローカルの変更 | upstream の出典 | upstream commit |
| --- | --- | --- |
| Ctrl+Space の設定化と関連ドキュメント | [PR #39](https://github.com/togatoga/karukan/pull/39) | `a86ad09`, `1434332`, `d71945e`, `e694727`, `9938738`, `8301c62`, `32dd3e4` |
| F6–F10 / Muhenkan 対応 | [PR #42](https://github.com/togatoga/karukan/pull/42) | `3b89300` |
| Shift+Space による半角スペース入力 | [PR #43](https://github.com/togatoga/karukan/pull/43) | `3cefd17` |
| macOS 候補ウィンドウの列揃え | [PR #50](https://github.com/togatoga/karukan/pull/50) | `c63f651` |
| 日付変換と候補順位・学習制御 | [PR #54](https://github.com/togatoga/karukan/pull/54) | `f869645`, `5bddfdf` |
| 最新 upstream の `LiveConversion.shown` への互換調整 | [PR #85](https://github.com/togatoga/karukan/pull/85) | `91d053c` |
| rebase 時の競合解消を保持する互換調整 | downstream 固有 | `bc71899` |
| upstream の記号・幅設定を採用しつつ既存の空白挙動を維持する互換調整 | downstream 固有 | `2d51fd2` |

PR #50 の変更は、確認時点では `candidate-window-column-align` ブランチの別 worktree にある。現在の `main` からパッチを生成する場合には含まれないため、必要ならその commit を別パッチとして扱う。

## 初回セットアップ

別の Mac では、まず fork を clone して `upstream` を登録する。

~~~bash
git clone git@github.com:asonas/karukan.git
cd karukan
git remote add upstream ssh://git@github.com/togatoga/karukan.git
git fetch --all --prune
git switch main
~~~

適用済みソースは `main` と分離した worktree に作る。

~~~bash
git worktree add -b applied/asonas .worktrees/applied main
~~~

## 既存パッチを適用する

適用済み worktree で、作業開始前の状態を確認してから `patches/*.patch` を適用する。この用途では `git am` を使わない。`git apply` はソースを変更するが、コミットや index は変更しない。

~~~bash
git status --short
for patch in patches/*.patch; do
  [ -e "$patch" ] || break
  git apply "$patch" || exit 1
done
git status --short
git diff --check
~~~

`git apply --3way` は競合時に index を変更することがあるため、通常はまず通常の `git apply` を使う。適用後のソース変更は、この worktree でビルド・テストするためのものであり、`main` にはコミットしない。

## upstream/main を取り込む標準フロー

適用済み worktree の変更を rebase し続けるのではなく、patch-only の `main` を upstream の先端へ移動し、適用済み worktree を作り直す。

~~~bash
git fetch upstream --prune
git rebase upstream/main
~~~

この `rebase` は `main` に残ったパッチ・運用文書のコミットを更新するために行う。適用済み worktree のソース変更を rebase 対象にしない。パッチと upstream の競合を解消する必要がある場合は、一時 queue worktree で解消し、パッチを再生成する。

## パッチを編集・再生成する

既存パッチを編集したり、upstream 更新との競合を解消したりする場合だけ、一時 queue worktree で `git am` を使う。

~~~bash
repo_root="$(git rev-parse --show-toplevel)"
git worktree add -b patch-queue-edit .worktrees/patch-queue-edit upstream/main
for patch in "$repo_root"/patches/*.patch; do
  git am --3way "$patch" || exit 1
done
~~~

一時コミット上でソースを修正し、必要な検証を行う。競合時は `git am --continue`、中止時は `git am --abort` を使う。問題を解消したら、upstream を基底にした一時コミット列からパッチを作り直す。

~~~bash
patch_output="$(mktemp -d)"
git format-patch --binary --output-directory "$patch_output" upstream/main..HEAD -- . ":(exclude)patches/**"
find "$patch_output" -maxdepth 1 -type f -name "*.patch" -print | sort
command rm patches/*.patch
command cp "$patch_output"/*.patch patches/
~~~

`git format-patch` はコミットを入力にするため、一時 queue worktree ではコミットが必要になる。生成した一時コミットは `main` に残さず、変更後の `patches/*.patch` だけを `main` に持ち帰る。

## コミット範囲

コミット対象はパッチと運用文書だけに限定する。適用済みソースを誤ってコミットしないよう、ステージ後のファイル名を確認する。

~~~bash
git add patches/*.patch patches/README.md patches/AGENTS.md
git diff --cached --name-only
git diff --cached --check -- patches/README.md patches/AGENTS.md
git status --short
git commit -m "Maintain Patch Queue As Canonical State"
~~~

ステージされたファイルに `karukan-im/` などのソースパスが含まれていたら、コミットを中止してステージ内容を見直す。パッチ適用後のソース変更は、パッチファイルを更新する一時 queue worktree 以外ではコミットしない。

## 検証

最低限、パッチ適用前後の差分検査と、変更範囲に近いテストを実行する。

~~~bash
git diff --check -- patches/README.md patches/AGENTS.md
mise exec -- cargo test -p karukan-im learning
~~~

macOS フロントエンドを変更した場合は、利用可能な環境で次も実行する。

~~~bash
make -C karukan-im/macos test
~~~

## push 前の確認

`main` の履歴を書き換えた場合は、push 前に次を確認する。push は人間が実行する。

~~~bash
git remote get-url origin
git branch --show-current
git --no-pager log --oneline upstream/main..HEAD
git push --set-upstream origin main
~~~

既存の `origin/main` を履歴書き換え後の `main` で置き換える場合は、対象とバックアップを確認したうえで `git push --force-with-lease origin main` を使う。

## 失敗時の扱い

`git apply` が失敗したら、失敗箇所と upstream 側の変更を確認してからパッチファイルを更新する。`git am` の一時作業が失敗した場合は、`git am --continue` または `git am --abort` を使う。問題を解消しないまま `--skip` してパッチ列を壊さない。
