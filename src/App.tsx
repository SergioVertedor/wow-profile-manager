import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import AcercaDe from "./AcercaDe";
import Instalaciones from "./Instalaciones";
import { pedirInfoApp, type InfoApp } from "./nucleo";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; info: InfoApp }
  | { tipo: "error"; mensaje: string };

/** Vistas de la ventana. Con dos basta un estado; cuando lleguen los flujos con pasos se medirá un router. */
type Vista = "instalaciones" | "acercaDe";

function App() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  const [vista, setVista] = useState<Vista>("instalaciones");

  useEffect(() => {
    pedirInfoApp()
      .then((info) => setEstado({ tipo: "listo", info }))
      .catch((error: unknown) => setEstado({ tipo: "error", mensaje: String(error) }));
  }, []);

  return (
    <main>
      <header>
        {estado.tipo === "cargando" && <p>{t("cargando")}</p>}
        {estado.tipo === "listo" && (
          <h1>{t("cabecera", { nombre: estado.info.nombre, version: estado.info.version })}</h1>
        )}
        {estado.tipo === "error" && <p role="alert">{t("errorNucleo", { mensaje: estado.mensaje })}</p>}
        <nav>
          <BotonVista vista="instalaciones" actual={vista} elegir={setVista} />
          {estado.tipo === "listo" && <BotonVista vista="acercaDe" actual={vista} elegir={setVista} />}
        </nav>
      </header>
      {vista === "instalaciones" && <Instalaciones />}
      {vista === "acercaDe" && estado.tipo === "listo" && <AcercaDe info={estado.info} />}
    </main>
  );
}

function BotonVista({
  vista,
  actual,
  elegir,
}: Readonly<{ vista: Vista; actual: Vista; elegir: (vista: Vista) => void }>) {
  const { t } = useTranslation();
  return (
    <button type="button" aria-pressed={vista === actual} onClick={() => elegir(vista)}>
      {t(`nav.${vista}`)}
    </button>
  );
}

export default App;
