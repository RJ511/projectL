import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./css/App.css";

// Inicializa o aplicativo React no elemento com id 'root'
const root = ReactDOM.createRoot(document.getElementById("root")); // Cria a raiz
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
