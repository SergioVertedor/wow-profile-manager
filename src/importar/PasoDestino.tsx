// Paso «Destino» (#63): uno o varios personajes de una misma instalación de la versión del juego del paquete,
// aunque sean de cuentas distintas. Se reutiliza la lista de la vista «Personajes» con casillas.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { etiquetaDeInstalacion, nombreDeProducto } from "../instalaciones/nombres";
import type { Deteccion, ResumenPaquete } from "../nucleo";
import Plantel from "../personajes/Plantel";
import { claveDePersonaje, numerosDeCuenta } from "../personajes/lista";
import {
  alternarDestino,
  claveDeDestino,
  marcadosEnCuenta,
  versionesDelProducto,
  type Destino,
} from "./importacion";
import css from "./Importar.module.css";

type Props = Readonly<{
  resumen: ResumenPaquete;
  deteccion: Deteccion;
  destinos: ReadonlyMap<string, Destino>;
  cambiar: (destinos: ReadonlyMap<string, Destino>) => void;
}>;

export default function PasoDestino({ resumen, deteccion, destinos, cambiar }: Props) {
  const { t } = useTranslation();
  const { producto, interfaz } = resumen.manifiesto.origen;
  const versiones = versionesDelProducto(deteccion, producto);
  // La versión se deduce de los destinos ya marcados: al volver a este paso se sigue en la misma instalación.
  const marcada = [...destinos.values()][0]?.pedido;
  const [claveVersion, setClaveVersion] = useState(
    marcada === undefined ? null : `${marcada.raiz}|${marcada.version}`,
  );
  const [carpetaCuenta, setCarpetaCuenta] = useState<string | null>(marcada?.cuenta ?? null);

  const entrada = versiones.find((v) => v.clave === claveVersion) ?? versiones[0];
  if (entrada === undefined) {
    return <p className={css.aviso}>{t("importar.sinInstalacion", { version: nombreDeProducto(t, producto) })}</p>;
  }
  const cuentas = entrada.version.cuentas.cuentas;
  const cuenta = cuentas.find((c) => c.carpeta === carpetaCuenta) ?? cuentas[0];
  const numeros = numerosDeCuenta(cuentas.map((c) => c.carpeta));
  const porDefecto = (carpeta: string) => {
    const numero = numeros.get(carpeta) ?? null;
    return numero === null ? carpeta : t("plantel.cuenta", { numero });
  };
  const etiquetaCuenta = (carpeta: string) => cuentas.find((c) => c.carpeta === carpeta)?.nombre ?? porDefecto(carpeta);
  const elegirVersion = (clave: string) => {
    setClaveVersion(clave);
    setCarpetaCuenta(null);
    // Todos los destinos tienen que ser de la misma instalación.
    cambiar(new Map());
  };

  return (
    <div className={css.destino}>
      <div className={css.columna}>
        {versiones.length > 1 && (
          <div className={css.versiones} role="group" aria-label={t("importar.instalacion")}>
            {versiones.map((v) => (
              <button
                key={v.clave}
                type="button"
                aria-pressed={v.clave === entrada.clave}
                title={v.instalacion.raiz}
                onClick={() => elegirVersion(v.clave)}
              >
                {etiquetaDeInstalacion(t, deteccion, v.instalacion)}
              </button>
            ))}
          </div>
        )}
        {entrada.version.interfaz !== null && entrada.version.interfaz !== interfaz && (
          <p className={css.aviso}>
            <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
            {t("importar.interfazDistinta", { paquete: interfaz, destino: entrada.version.interfaz })}
          </p>
        )}
        <p className={css.nota}>{t("importar.elegirDestinos")}</p>
        <p aria-live="polite">{t("importar.marcados", { count: destinos.size })}</p>
        {destinos.size > 0 && (
          <ul className={css.marcados}>
            {[...destinos.values()].map((d) => (
              <li key={claveDeDestino(d.pedido.cuenta, `${d.pedido.reino}/${d.pedido.personaje}`)}>
                {d.nombre} <span className={css.secundarioTexto}>{etiquetaCuenta(d.pedido.cuenta)}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
      {cuenta === undefined ? (
        <p className={css.nota}>{t("personajes.sinCuentas")}</p>
      ) : (
        <Plantel
          key={`${entrada.clave}|${cuenta.carpeta}`}
          cuentas={cuentas}
          cuenta={cuenta}
          producto={entrada.version.producto}
          etiquetaCuenta={etiquetaCuenta}
          cuentaPorDefecto={porDefecto(cuenta.carpeta)}
          elegirCuenta={setCarpetaCuenta}
          personaje={null}
          marcados={marcadosEnCuenta(destinos, cuenta.carpeta)}
          elegirPersonaje={(clave) => {
            const personaje = cuenta.personajes.find((p) => claveDePersonaje(p) === clave);
            if (personaje !== undefined) {
              cambiar(alternarDestino(destinos, entrada, cuenta.carpeta, personaje));
            }
          }}
        />
      )}
    </div>
  );
}
