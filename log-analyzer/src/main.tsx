import React from "react";
import ReactDOM from "react-dom/client";
import { enableMapSet } from "immer";
import App from "./App";
import { AppearanceProvider } from "./theme/AppearanceProvider";
import "./index.css"; // <--- 关键！必须引入 CSS

// 启用 Immer 的 Map/Set 支持（taskStore 使用 Map 索引）
enableMapSet();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppearanceProvider>
      <App />
    </AppearanceProvider>
  </React.StrictMode>
);
