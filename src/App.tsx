import { useEffect, useState } from "react";
import { pedirInfoApp, textoCabecera, type InfoApp } from "./nucleo";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; info: InfoApp }
  | { tipo: "error"; mensaje: string };

function App() {
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });

  useEffect(() => {
    pedirInfoApp()
      .then((info) => setEstado({ tipo: "listo", info }))
      .catch((error: unknown) => setEstado({ tipo: "error", mensaje: String(error) }));
  }, []);

  return (
    <main>
      {estado.tipo === "cargando" && <p>Cargando…</p>}
      {estado.tipo === "listo" && <h1>{textoCabecera(estado.info)}</h1>}
      {estado.tipo === "error" && <p role="alert">No se pudo contactar con el núcleo: {estado.mensaje}</p>}
    </main>
  );
}

export default App;
