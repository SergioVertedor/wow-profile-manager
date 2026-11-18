// La detección de instalaciones, compartida por «Personajes» e «Instalaciones» (spec 018). Vive en `App` para que
// cambiar de vista no vuelva a leer el disco.
import { useCallback, useEffect, useRef, useState } from "react";
import { pedirDeteccion, type Deteccion } from "../nucleo";

export type EstadoDeteccion =
  | { tipo: "cargando" }
  | { tipo: "listo"; deteccion: Deteccion }
  | { tipo: "error"; error: unknown };

export type UsoDeteccion = Readonly<{
  estado: EstadoDeteccion;
  /** Vuelve a detectar sin pasar por «cargando»: lo que se ve se queda hasta que llega lo nuevo. */
  recargar: () => Promise<void>;
  /** Cambia la detección en memoria (tras renombrar), sin leer el disco ni perder lo elegido. */
  actualizar: (cambio: (deteccion: Deteccion) => Deteccion) => void;
}>;

export function useDeteccion(): UsoDeteccion {
  const [estado, setEstado] = useState<EstadoDeteccion>({ tipo: "cargando" });
  // Cada recarga y cada cambio en memoria llevan un número: una respuesta que llega después de otro más reciente se
  // descarta, para que una detección vieja no pise lo nuevo (por ejemplo un nombre recién guardado).
  const ultima = useRef(0);

  const recargar = useCallback(() => {
    const esta = ++ultima.current;
    return pedirDeteccion()
      .then((deteccion) => {
        if (esta === ultima.current) {
          setEstado({ tipo: "listo", deteccion });
        }
      })
      .catch((error: unknown) => {
        if (esta === ultima.current) {
          setEstado({ tipo: "error", error });
        }
      });
  }, []);

  const actualizar = useCallback((cambio: (deteccion: Deteccion) => Deteccion) => {
    ultima.current++;
    setEstado((actual) => (actual.tipo === "listo" ? { tipo: "listo", deteccion: cambio(actual.deteccion) } : actual));
  }, []);

  useEffect(() => {
    void recargar();
  }, [recargar]);

  return { estado, recargar, actualizar };
}
