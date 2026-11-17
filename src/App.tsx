import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { pedirInfoApp, type InfoApp } from "./nucleo";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; info: InfoApp }
  | { tipo: "error"; mensaje: string };

function App() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });

  useEffect(() => {
    pedirInfoApp()
      .then((info) => setEstado({ tipo: "listo", info }))
      .catch((error: unknown) => setEstado({ tipo: "error", mensaje: String(error) }));
  }, []);

  return (
    <main>
      {estado.tipo === "cargando" && <p>{t("cargando")}</p>}
      {estado.tipo === "listo" && (
        <h1>{t("cabecera", { nombre: estado.info.nombre, version: estado.info.version })}</h1>
      )}
      {estado.tipo === "error" && <p role="alert">{t("errorNucleo", { mensaje: estado.mensaje })}</p>}
    </main>
  );
}

export default App;
