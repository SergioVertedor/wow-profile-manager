// «Abrir carpeta» del personaje elegido (spec 026): el núcleo la busca y la abre en el explorador. La interfaz solo
// pasa la identidad del personaje, nunca una ruta. Solo lee.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import { abrirCarpetaPersonaje, type PersonajePedido } from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import estiloDialogo from "../estilo/Dialogo.module.css";
import css from "./AbrirCarpeta.module.css";

type Props = Readonly<{ pedido: PersonajePedido }>;

export default function AbrirCarpeta({ pedido }: Props) {
  const { t } = useTranslation();
  const [error, setError] = useState<unknown>(null);
  const abrir = () => {
    setError(null);
    abrirCarpetaPersonaje(pedido).catch(setError);
  };
  return (
    <>
      <button type="button" className={css.boton} onClick={abrir}>
        <FolderOpen aria-hidden size={15} strokeWidth={1.75} />
        {t("ficha.abrirCarpeta")}
      </button>
      {error !== null && (
        <p role="alert" className={estiloDialogo.alerta}>
          {mensajeDeError(t, error)}
        </p>
      )}
    </>
  );
}
