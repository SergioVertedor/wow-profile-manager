// Estado de la pestaña Diagnóstico: el interruptor y las líneas del registro en vivo. Las líneas de antes de abrir la
// pestaña vienen del núcleo (las guarda en memoria); las nuevas, del evento. Se escucha el evento ANTES de pedir las
// guardadas para no perder ninguna entre medias, y se juntan por su número de orden sin repetir.
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { cambiarRegistro, EVENTO_LINEA, pedirDiagnostico, type LineaDiagnostico } from "../nucleo";

/** Hasta que el núcleo diga cuántas guarda, no se recorta: solo pueden llegar las de unos milisegundos. */
const SIN_MAXIMO = Number.POSITIVE_INFINITY;

/** Junta dos tandas de líneas en orden, sin repetir ninguna y con las `maximo` más nuevas. */
export function juntarLineas(
  actuales: readonly LineaDiagnostico[],
  nuevas: readonly LineaDiagnostico[],
  maximo: number,
): LineaDiagnostico[] {
  const porNumero = new Map<number, LineaDiagnostico>();
  for (const linea of [...actuales, ...nuevas]) {
    porNumero.set(linea.n, linea);
  }
  return [...porNumero.values()].sort((a, b) => a.n - b.n).slice(-maximo);
}

type Interruptor = { tipo: "cargando" } | { tipo: "listo"; guarda: boolean } | { tipo: "cambiando"; guarda: boolean };

export function useDiagnostico() {
  const [lineas, setLineas] = useState<LineaDiagnostico[]>([]);
  const [interruptor, setInterruptor] = useState<Interruptor>({ tipo: "cargando" });
  const [error, setError] = useState<unknown>(null);
  const maximo = useRef(SIN_MAXIMO);

  useEffect(() => {
    let viva = true;
    const dejarDeEscuchar = listen<LineaDiagnostico>(EVENTO_LINEA, (evento) => {
      setLineas((actuales) => juntarLineas(actuales, [evento.payload], maximo.current));
    });
    dejarDeEscuchar
      .then(() => pedirDiagnostico())
      .then((estado) => {
        if (viva) {
          maximo.current = estado.maximo;
          setInterruptor({ tipo: "listo", guarda: estado.guarda });
          setLineas((actuales) => juntarLineas(actuales, estado.lineas, estado.maximo));
        }
      })
      .catch((motivo: unknown) => viva && setError(motivo));
    return () => {
      viva = false;
      void dejarDeEscuchar.then((dejar) => dejar());
    };
  }, []);

  const cambiar = useCallback((guarda: boolean) => {
    setError(null);
    setInterruptor({ tipo: "cambiando", guarda });
    cambiarRegistro(guarda)
      .then(() => setInterruptor({ tipo: "listo", guarda }))
      .catch((motivo: unknown) => {
        setInterruptor({ tipo: "listo", guarda: !guarda });
        setError(motivo);
      });
  }, []);

  return { lineas, interruptor, error, cambiar };
}
