// Paso «Vista previa» (#63): el plan del núcleo tal cual, por personaje y en los grupos de `docs/flows.md`. Pedirlo
// no escribe nada. Al pie, «Aplicar» (#63).
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import { TriangleAlert } from "lucide-react";
import ListaQuitadas, { type TextoDeMotivo } from "../comun/ListaQuitadas";
import {
  pedirPlanImportacion,
  type AvisoFichero,
  type ConflictoPerfil,
  type DecisionPerfil,
  type Elemento,
  type FicheroPlan,
  type Motivo,
  type PedidoImportacion,
  type PlanPersonaje,
  type ResultadoImportacion,
  type ResumenPaquete,
  type VistaPrevia as Plan,
} from "../nucleo";
import AplicarImportacion from "./AplicarImportacion";
import { mensajeDeError } from "../textos/errores";
import { tamanoLegible } from "../textos/formato";
import { etiquetaDeCategoria, etiquetaDePieza } from "./etiquetas";
import { delGrupo, GRUPOS, type Destino } from "./importacion";
import css from "./Importar.module.css";

type Estado = { tipo: "cargando" } | { tipo: "listo"; vista: Plan } | { tipo: "error"; error: unknown };

type Props = Readonly<{
  resumen: ResumenPaquete;
  destinos: readonly Destino[];
  pedido: PedidoImportacion;
  /** Decide qué hacer con un perfil que ya existe: el plan se vuelve a pedir con la decisión. */
  decidirPerfil: (decision: DecisionPerfil) => void;
  alAplicar: (resultados: ResultadoImportacion[]) => void;
}>;

export default function VistaPrevia({ resumen, destinos, pedido, decidirPerfil, alAplicar }: Props) {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  // Sube tras un fallo al aplicar: el plan se vuelve a pedir, porque el disco pudo cambiar. El fallo se sigue enseñando.
  const [vuelta, setVuelta] = useState(0);
  const [falloAlAplicar, setFalloAlAplicar] = useState<unknown>(null);
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
  }, [resumen.sha256, peticion, vuelta]);

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
  const alerta =
    falloAlAplicar === null ? null : (
      <p role="alert" className={css.alerta}>
        {mensajeDeError(t, falloAlAplicar)}
      </p>
    );
  const { interfaz } = resumen.manifiesto.origen;
  const nombreDe = (p: PlanPersonaje) =>
    destinos.find((d) => d.pedido.cuenta === p.cuenta && d.pedido.personaje === p.personaje)?.nombre ?? p.personaje;
  const etiquetaDeEntrada = (id: string) => {
    const entrada = resumen.entradas.find((e) => e.id === id);
    if (entrada === undefined) {
      return id;
    }
    return entrada.categoria === "config-addon"
      ? `${entrada.addon ?? ""}: ${etiquetaDePieza(t, entrada)}`
      : etiquetaDeCategoria(t, entrada.categoria, entrada.deCuenta);
  };
  return (
    <div className={css.columna}>
      {alerta}
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
          pedido={pedido}
          decidirPerfil={decidirPerfil}
        />
      ))}
      <AplicarImportacion
        resumen={resumen}
        vista={vista}
        destinos={destinos}
        pedido={pedido}
        nombreDe={nombreDe}
        alAplicar={alAplicar}
        alFallar={(error) => {
          setFalloAlAplicar(error);
          setEstado({ tipo: "cargando" });
          setVuelta((v) => v + 1);
        }}
      />
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
  pedido,
  decidirPerfil,
}: Readonly<{
  plan: PlanPersonaje;
  nombreDe: (p: PlanPersonaje) => string;
  etiquetaDeEntrada: (id: string) => string;
  vista: Plan;
  pedido: PedidoImportacion;
  decidirPerfil: (decision: DecisionPerfil) => void;
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
      {plan.conflictosPerfil.map((conflicto) => (
        <PerfilQueExiste
          key={conflicto.entrada}
          conflicto={conflicto}
          cuenta={plan.cuenta}
          etiqueta={etiquetaDeEntrada(conflicto.entrada)}
          pedido={pedido}
          decidir={decidirPerfil}
        />
      ))}
      {GRUPOS.map((grupo) => {
        const ficheros = delGrupo(plan.ficheros, grupo);
        return (
          ficheros.length > 0 && (
            <div key={grupo}>
              <h3 className={css.grupo}>{t(`importar.grupos.${grupo}`)}</h3>
              <ul className={css.ficheros}>
                {ficheros.map((f) => (
                  <Fichero key={f.destino} fichero={f} escritaPor={escritaPor} etiqueta={etiquetaDeEntrada(f.entrada)} />
                ))}
              </ul>
            </div>
          )
        );
      })}
      {plan.addons.length > 0 && (
        <div>
          <h3 className={css.grupo}>{t("importar.grupos.addons")}</h3>
          <ul className={css.ficheros}>
            {plan.addons.map((a) => (
              <li key={a.entrada}>
                {a.addon}{" "}
                <span className={css.secundarioTexto}>
                  {a.yaInstalado ? t("importar.addonYaInstalado") : t("importar.addonSeInstala")}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {plan.noAplicables.length > 0 && (
        <div>
          <h3 className={css.grupo}>{t("importar.grupos.noAplicables")}</h3>
          <ul className={css.ficheros}>
            {plan.noAplicables.map((n) => (
              <li key={`${n.entrada}-${n.motivo}`}>
                {etiquetaDeEntrada(n.entrada)}{" "}
                <span className={css.secundarioTexto}>{t(`importar.noAplicable.${n.motivo}`)}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {plan.ficheros.length === 0 &&
        plan.addons.length === 0 &&
        plan.conflictos.length === 0 &&
        plan.conflictosPerfil.length === 0 && (
        <p className={css.nota}>{t("importar.nadaQueImportar")}</p>
      )}
    </section>
  );
}

function Fichero({
  fichero,
  escritaPor,
  etiqueta,
}: Readonly<{ fichero: FicheroPlan; escritaPor: (carpeta: string) => string; etiqueta: string }>) {
  const { t, i18n } = useTranslation();
  const tamano = (bytes: number) => tamanoLegible(bytes, i18n.language);
  return (
    <li className={css.fichero}>
      <p className={css.lineaFichero}>
        <span className={css.categoria}>
          {fichero.categoria === "config-addon" ? etiqueta : etiquetaDeCategoria(t, fichero.categoria, fichero.deCuenta)}
        </span>
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
      {fichero.avisos.map((aviso) => (
        <Aviso key={`${aviso.tipo}-${aviso.addon}`} texto={textoDeAviso(t, aviso)} />
      ))}
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
        <div className={css.secundarioTexto}>
          <ListaQuitadas grupos={fichero.apartadas} textoDe={textoDeApartadas} />
        </div>
      )}
    </li>
  );
}

/** Al importar, los addons que faltan sí se escriben (#63, decisión 9): su frase es propia. */
function textoDeApartadas(motivo: Motivo): TextoDeMotivo {
  return motivo === "addonNoInstalado" ? "importar.addonsQueFaltan" : `motivos.${motivo}`;
}

function ElementoCambiado({ elemento }: Readonly<{ elemento: Elemento }>) {
  const { t } = useTranslation();
  const valor = (texto: string | undefined) => texto ?? "—";
  return (
    <>
      <span className={css.cambio} data-cambio={elemento.cambio}>
        {t(`importar.cambio.${elemento.cambio}`)}
      </span>{" "}
      <strong>{nombreDeElemento(t, elemento)}</strong>
      {elemento.cambio === "cambiado" && (
        <span className={css.secundarioTexto}>
          {" "}
          {valor(elemento.antes)} → {valor(elemento.despues)}
        </span>
      )}
    </>
  );
}

/** Un perfil, el perfil de un personaje o una variable de la configuración de addons; el resto, tal cual. */
function nombreDeElemento(t: TFunction, elemento: Elemento): string {
  switch (elemento.clase) {
    case "perfil":
      return t("importar.elemento.perfil", { nombre: elemento.nombre });
    case "perfilDe":
      return t("importar.elemento.perfilDe", { nombre: elemento.nombre });
    case "diseno":
      return t("importar.elemento.diseno", { nombre: elemento.nombre });
    case "disenoActivo":
      return t("importar.elemento.disenoActivo", { nombre: elemento.nombre });
    default:
      return elemento.nombre;
  }
}

function textoDeAviso(t: TFunction, aviso: AvisoFichero): string {
  if (aviso.tipo === "faltaAddon") {
    return t("importar.avisos.faltaAddon", { addon: aviso.addon });
  }
  return t("importar.avisos.versionNoMedida", {
    addon: aviso.addon,
    instalada: aviso.instalada ?? t("importar.avisos.sinVersion"),
    medidas: aviso.medidas.join(", "),
  });
}

/** Un perfil AceDB que ya existe y es distinto: renombrar (no toca a nadie más) o sobrescribir. */
function PerfilQueExiste({
  conflicto,
  cuenta,
  etiqueta,
  pedido,
  decidir,
}: Readonly<{
  conflicto: ConflictoPerfil;
  cuenta: string;
  etiqueta: string;
  pedido: PedidoImportacion;
  decidir: (decision: DecisionPerfil) => void;
}>) {
  const { t } = useTranslation();
  const nombre = useId();
  const usadoPor = useId();
  const elegida = pedido.perfiles.find((d) => d.cuenta === cuenta && d.entrada === conflicto.entrada)?.accion;
  const avisaUsadoPor = conflicto.usadoPor.length > 0;
  const opcion = (accion: DecisionPerfil["accion"], texto: string, descrita?: string) => (
    <label>
      <input
        type="radio"
        name={nombre}
        aria-describedby={descrita}
        checked={elegida === accion}
        onChange={() => decidir({ cuenta, entrada: conflicto.entrada, accion })}
      />
      {texto}
    </label>
  );
  return (
    <fieldset className={css.decision}>
      <legend className={css.conflicto}>
        {t("importar.perfiles.existe", { entrada: etiqueta, perfil: conflicto.perfil })}
      </legend>
      {opcion("renombrar", t("importar.perfiles.renombrar", { propuesto: conflicto.propuesto }))}
      {opcion(
        "sobrescribir",
        t("importar.perfiles.sobrescribir", { perfil: conflicto.perfil }),
        avisaUsadoPor ? usadoPor : undefined,
      )}
      {avisaUsadoPor && (
        <p id={usadoPor} className={css.secundarioTexto}>
          {t("importar.perfiles.usadoPor", { personajes: conflicto.usadoPor.join(", ") })}
        </p>
      )}
    </fieldset>
  );
}
