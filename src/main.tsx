import React from "react";
import ReactDOM from "react-dom/client";
import { I18nextProvider } from "react-i18next";
import App from "./App";
import "./estilo/base.css";
import { iniciarTextos } from "./textos";

// No se pinta nada hasta saber el idioma: así ningún texto aparece primero en un idioma y luego en otro.
void iniciarTextos().then((textos) => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <I18nextProvider i18n={textos}>
        <App />
      </I18nextProvider>
    </React.StrictMode>,
  );
});
