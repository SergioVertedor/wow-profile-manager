// Paso «Vista previa» (spec 024): el plan del núcleo tal cual, por personaje y en los grupos de `docs/flujos.md`. Pedirlo
// no escribe nada. Aplicar llega en la entrega 2 de #63.
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import {
  pedirPlanImportacion,
  type Elemento,
  type FicheroPlan,
  type PedidoImportacion,
  type PlanPersonaje,
  type ResumenPaquete,
  type VistaPrevia as Plan,
} from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import { tamanoLegible } from "../textos/formato";
import { etiquetaDeCategoria } from "./etiquetas";
import { delGrupo, GRUPOS, type Destino } from "./importacion";
import css from "./Importar.module.css";

type Estado = { tipo: "cargando" } | { tipo: "listo"; vista: Plan } | { tipo: "error"; error: unknown };

type Props = Readonly<{ resumen: ResumenPaquete; destinos: readonly Destino[]; pedido: PedidoImportacion }>;

export default function VistaPrevia({ resumen, destinos, pedido }: Props) {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  // El plan depende de lo elegido en los pasos anteriores: se compara por valor, no por identidad.
  const peticion = JSON.stringify({ personajes: destinos.map((d) => d.pedido), pedido });

  useEffect(() => {
    let vigente = true;
    const { personajes, pedido: elegido } = JSON.parse(peticion) as {
      personajes: Destino["pedido"][];
      pedido: PedidoImportacion;
    };
    pedirPlanImportacion(resumen.sha256, personajes, elegido)
      .then((vista) => vigente && setEstado({ tipo: "listo", vista }))
      .catch((error: unknown) => vigente && setEstado({ tipo: "error", error }));
    return () => {
      vigente = false;
    };
  }, [resumen.sha256, peticion]);

  if (estado.tipo === "cargando") {
    return <p className={css.nota}>{t("importar.calculando")}</p>;
  }
  if (estado.tipo === "error") {
    return (
      <p role="alert" className={css.alerta}>
        {mensajeDeError(t, estado.error)}
      </p>
    );
  }
  const { vista } = estado;
  const { interfaz } = resumen.manifiesto.origen;
  const nombreDe = (p: PlanPersonaje) =>
    destinos.find((d) => d.pedido.cuenta === p.cuenta && d.pedido.personaje === p.personaje)?.nombre ?? p.personaje;
  const etiquetaDeEntrada = (id: string) => {
    const entrada = resumen.entradas.find((e) => e.id === id);
    return entrada === undefined ? id : etiquetaDeCategoria(t, entrada.categoria, entrada.deCuenta);
  };
  return (
    <div className={css.columna}>
      {vista.juegoAbierto && <Aviso texto={t("importar.juegoAbierto")} />}
      {vista.interfazDestino !== undefined && vista.interfazDestino !== interfaz && (
        <Aviso texto={t("importar.interfazDistinta", { paquete: interfaz, destino: vista.interfazDestino })} />
      )}
      {vista.plan.incompatibles.length > 0 && (
        <section aria-label={t("importar.grupos.incompatibles")}>
          <h3 className={css.grupo}>{t("importar.grupos.incompatibles")}</h3>
          <ul className={css.ficheros}>
            {vista.plan.incompatibles.map((i) => (
              <li key={i.entrada}>
                {etiquetaDeCategoria(t, i.categoria, false)}{" "}
                <span className={css.secundarioTexto}>{t(`importar.incompatible.${i.motivo}`)}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      {vista.plan.personajes.map((p) => (
        <Personaje
          key={`${p.cuenta}|${p.reino}|${p.personaje}`}
          plan={p}
          nombreDe={nombreDe}
          etiquetaDeEntrada={etiquetaDeEntrada}
          vista={vista}
        />
      ))}
      <p className={css.nota}>{t("importar.soloVistaPrevia")}</p>
    </div>
  );
}

function Aviso({ texto }: Readonly<{ texto: string }>) {
  return (
    <p className={css.aviso}>
      <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
      {texto}
    </p>
  );
}

function Personaje({
  plan,
  nombreDe,
  etiquetaDeEntrada,
  vista,
}: Readonly<{
  plan: PlanPersonaje;
  nombreDe: (p: PlanPersonaje) => string;
  etiquetaDeEntrada: (id: string) => string;
  vista: Plan;
}>) {
  const { t } = useTranslation();
  const nombre = nombreDe(plan);
  const escritaPor = (carpeta: string) => {
    const quien = vista.plan.personajes.find((otro) => otro.cuenta === plan.cuenta && otro.personaje === carpeta);
    return quien === undefined ? carpeta : nombreDe(quien);
  };
  return (
    <section className={css.personaje} aria-label={nombre}>
      <h2 className={css.nombrePersonaje}>{nombre}</h2>
      {plan.conflictos.length > 0 && (
        <div>
          <h3 className={css.grupo}>{t("importar.grupos.conflictos")}</h3>
          <ul className={css.ficheros}>
            {plan.conflictos.map((c) => (
              <li key={c} className={css.conflicto}>
                {t("importar.sinDecidir", { entrada: etiquetaDeEntrada(c) })}
              </li>
            ))}
          </ul>
        </div>
      )}
      {GRUPOS.map((grupo) => {
        const ficheros = delGrupo(plan.ficheros, grupo);
        return (
          ficheros.length > 0 && (
            <div key={grupo}>
              <h3 className={css.grupo}>{t(`importar.grupos.${grupo}`)}</h3>
              <ul className={css.ficheros}>
                {ficheros.map((f) => (
                  <Fichero key={f.entrada} fichero={f} escritaPor={escritaPor} />
                ))}
              </ul>
            </div>
          )
        );
      })}
      {plan.ficheros.length === 0 && plan.conflictos.length === 0 && (
        <p className={css.nota}>{t("importar.nadaQueImportar")}</p>
      )}
    </section>
  );
}

function Fichero({ fichero, escritaPor }: Readonly<{ fichero: FicheroPlan; escritaPor: (carpeta: string) => string }>) {
  const { t, i18n } = useTranslation();
  const tamano = (bytes: number) => tamanoLegible(bytes, i18n.language);
  return (
    <li className={css.fichero}>
      <p className={css.lineaFichero}>
        <span className={css.categoria}>{etiquetaDeCategoria(t, fichero.categoria, fichero.deCuenta)}</span>
        <span className={css.tamano}>
          {fichero.bytesAntes === undefined
            ? tamano(fichero.bytesDespues)
            : `${tamano(fichero.bytesAntes)} → ${tamano(fichero.bytesDespues)}`}
        </span>
      </p>
      {fichero.deCuenta && fichero.escritaEn === undefined && (
        <p className={css.secundarioTexto}>{t("importar.afectaCuenta")}</p>
      )}
      {fichero.escritaEn !== undefined && (
        <p className={css.secundarioTexto}>{t("importar.escritaEn", { personaje: escritaPor(fichero.escritaEn) })}</p>
      )}
      {fichero.elementos.length > 0 && (
        <ul className={css.elementos}>
          {fichero.elementos.map((e, i) => (
            // Una misma clave puede salir quitada y añadida: el índice distingue.
            <li key={`${i}-${e.nombre}`}>
              <ElementoCambiado elemento={e} />
            </li>
          ))}
        </ul>
      )}
      {fichero.apartadas.length > 0 && (
        <p className={css.secundarioTexto}>
          {t(fichero.categoria === "opciones" ? "importar.opcionesApartadas" : "importar.addonsQueFaltan", {
            lista: fichero.apartadas.join(", "),
          })}
        </p>
      )}
    </li>
  );
}

function ElementoCambiado({ elemento }: Readonly<{ elemento: Elemento }>) {
  const { t } = useTranslation();
  const valor = (texto: string | undefined) => texto ?? "—";
  return (
    <>
      <span className={css.cambio} data-cambio={elemento.cambio}>
        {t(`importar.cambio.${elemento.cambio}`)}
      </span>{" "}
      <strong>{elemento.nombre}</strong>
      {elemento.cambio === "cambiado" && (
        <span className={css.secundarioTexto}>
          {" "}
          {valor(elemento.antes)} → {valor(elemento.despues)}
        </span>
      )}
    </>
  );
}
