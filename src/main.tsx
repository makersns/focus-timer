import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import MiniTimer from "./components/MiniTimer";
import TrayPopover from "./components/TrayPopover";
import "./styles.css";

const params = new URLSearchParams(window.location.search);
const view = params.get("view") ?? params.get("window");
const Surface = view === "mini" ? MiniTimer : view === "tray" ? TrayPopover : App;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Surface />
  </React.StrictMode>,
);
