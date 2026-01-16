<p align="center">
  <a href="https://github.com/wlswo/xray">
	<picture>
		<source media="(prefers-color-scheme: dark)" srcset="./assets/xray-logo.png">
		<source media="(prefers-color-scheme: light)" srcset="./assets/xray-logo.png">
		<img alt="Wave Terminal Logo" src="./assets/wave-light.png" width="100%">
	</picture>
  </a>
  <br/>
</p>

<img srcset="./assets/line-gradient.svg" alt="line break" width="100%" height="3px">

<br/>
<div align="center">

[English](./README.md) · [한국어](./README.KR.md) · [简体中文](./README_CN.md) · **日本語**

</div>

<img srcset="./assets/line-gradient.svg" alt="line break" width="100%" height="3px">

<br/><br/>

<div align="center">
  <table>
    <tr>
      <td align="center">
        <img src="https://github.com/user-attachments/assets/e15d9ae4-d06d-4130-9564-3e446bc4b6db" width="100%" />
        <br />
        <em>Webページの要素キャプチャ</em>
      </td>
      <td align="center">
        <img src="https://github.com/user-attachments/assets/a2f2ce9c-12ae-48af-a5d4-655d2d90baef" width="85%" />
        <br />
        <em>macOSプログラムの要素キャプチャ</em>
      </td>
    </tr>
  </table>
</div>

</br>

汎用 UI キャプチャおよびインスペクションツール。

Chrome デベロッパー ツールのインスペクターのように, OS 全体の UI 要素を検査およびキャプチャするための macOS デスクトップ アプリケーションです。

## 主な機能

- **グローバルインスペクター:** 画面上のウィンドウ、ボタン、または UI 要素にマウスを合わせると、ハイライト表示されます。
- **スマートキャプチャ:** ハイライト된 要素をクリックすると、即座にクリップボードにキャプチャされます。
- **システムトレイ統合:** バックグラウンドで静かに実行されます。
- **グローバルショートカット:** 必要に応じてインスペクターを起動できます。

## セットアップとインストール

1.  **リポジトリをクローンする**
2.  **依存関係のインストール:**
    ```bash
    npm install
    ```
3.  **開発ビル드の実行:**
    ```bash
    npm run tauri dev
    ```

## プロダクションビル드

本番環境用のアプリケーションパッケージをビル드하려면：

1.  **標準ビル드:**
    ```bash
    npm run tauri build
    ```

2.  **バージョン指定ビル드:**
    ビル드前に新しいバージョンを指定したい場合：
    ```bash
    npm run build:to --new_version=1.1.0
    ```
    *このコマンドは `package.json` と `Cargo.toml` を指定されたバージョンに更新し、ビル드プロセスを実行します。*

生成된 アプリケーションパッケージは `src-tauri/target/release/bundle/` に配置されます。

## 使用方法

1.  **権限の付与:**
    *   初回起動時に、アプリ（または開発モードで実行している場合はターミナル）に **アクセシビリティ (Accessibility)** と **画面収録 (Screen Recording)** の権限を付与する必要があります。
    *   アプリが動作しない場合は、*システム設定 > プライバシーとセキュリティ* を確認し、権限が有効になっていることを確認してください。

2.  **インスペクションの開始:**
    *   アプリはバックグラウンドで起動します（メニューバーのアイコンを確認してください）。
    *   <kbd>Cmd</kbd> + <kbd>Shift</kbd> + <kbd>X</kbd> を押してオーバーレイを有効にします。

3.  **キャプチャ:**
    *   マウスを動かして目的の UI 要素をハイライトします。
    *   **クリック** してキャプチャします。
    *   オーバーレイが閉じ、スクリーンショットが **クリップボード** に保存されます。任意の場所に貼り付けてください (<kbd>Cmd</kbd> + <kbd>V</kbd>)。

4.  **終了:**
    *   メニューバーのトレイアイコンをクリックし、**Quit** を選択します。

## アーキテクチャ

- **フロントエンド:** React + TypeScript (視覚적 オーバーレイ)
- **バックエンド:** Rust (Tauri, Accessibility API, CoreGraphics)
- **状態管理:** Tauri イベント (`element-hover`)

## ライセンス

[MIT](LICENSE)