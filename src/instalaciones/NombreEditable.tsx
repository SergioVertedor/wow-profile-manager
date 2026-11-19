// Un nombre propio que se puede cambiar en el sitio (spec 018): el texto con un lápiz al lado; el lápiz lo convierte en
// un campo. Intro guarda, Escape cancela y un campo vacío borra el nombre. Si el núcleo lo rechaza, se dice por qué y
// el campo sigue abierto. Al cerrar, el foco vuelve al lápiz.
import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Check, Pencil, X } from "lucide-react";
import { mensajeDeError } from "../textos/errores";
import css from "./NombreEditable.module.css";

type Props = Readonly<{
  /** Qué se nombra, para el lector de pantalla: «Nombre de la instalación». */
  etiqueta: string;
  /** El nombre propio actual; `null` si no tiene. */
  valor: string | null;
  /** Lo que se ve cuando no tiene nombre («Instalación 2»), también como pista del campo. */
  porDefecto: string;
  guardar: (nombre: string) => Promise<void>;
  /** Cómo se pinta el nombre cuando no se edita. */
  children: ReactNode;
}>;

export default function NombreEditable({ etiqueta, valor, porDefecto, guardar, children }: Props) {
  const { t } = useTranslation();
  const [borrador, setBorrador] = useState<string | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [guardando, setGuardando] = useState(false);
  const lapiz = useRef<HTMLButtonElement>(null);
  const devolverFoco = useRef(false);
  const idError = useId();

  useEffect(() => {
    if (borrador === null && devolverFoco.current) {
      devolverFoco.current = false;
      lapiz.current?.focus();
    }
  }, [borrador]);

  if (borrador === null) {
    return (
      <div className={css.fila}>
        {children}
        <button
          ref={lapiz}
          type="button"
          className={css.lapiz}
          aria-label={t("nombre.editar", { que: etiqueta })}
          title={t("nombre.editar", { que: etiqueta })}
          onClick={() => setBorrador(valor ?? "")}
        >
          <Pencil aria-hidden size={14} strokeWidth={2} />
        </button>
      </div>
    );
  }

  const cerrar = () => {
    devolverFoco.current = true;
    setBorrador(null);
    setError(null);
  };
  // Mientras se guarda no se cancela: si el guardado fallara, el error no tendría dónde verse.
  const cancelar = () => {
    if (!guardando) {
      cerrar();
    }
  };
  const enviar = () => {
    setGuardando(true);
    guardar(borrador)
      .then(cerrar)
      .catch((motivo: unknown) => setError(motivo))
      .finally(() => setGuardando(false));
  };
  return (
    <form
      className={css.formulario}
      onSubmit={(evento) => {
        evento.preventDefault();
        if (!guardando) {
          enviar();
        }
      }}
      onKeyDown={(evento) => {
        if (evento.key === "Escape") {
          cancelar();
        }
      }}
    >
      <div className={css.fila}>
        <input
          className={css.campo}
          aria-label={etiqueta}
          aria-invalid={error !== null}
          aria-describedby={error === null ? undefined : idError}
          placeholder={porDefecto}
          value={borrador}
          readOnly={guardando}
          // El campo aparece porque se ha pulsado el lápiz: el foco va donde se va a escribir.
          autoFocus
          onChange={(evento) => setBorrador(evento.target.value)}
        />
        <button type="submit" className={css.lapiz} aria-disabled={guardando} aria-label={t("nombre.guardar")}>
          <Check aria-hidden size={15} strokeWidth={2.25} />
        </button>
        <button type="button" className={css.lapiz} onClick={cancelar} aria-label={t("nombre.cancelar")}>
          <X aria-hidden size={15} strokeWidth={2.25} />
        </button>
      </div>
      {error !== null && (
        <p id={idError} role="alert" className={css.alerta}>
          {mensajeDeError(t, error)}
        </p>
      )}
    </form>
  );
}
