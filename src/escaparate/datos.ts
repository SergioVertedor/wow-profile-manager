// Datos FICTICIOS del escaparate (`escaparate.html`): la app real pintada con el núcleo simulado, para las capturas
// del README. Cuenta, personajes, rutas y paquetes son inventados; la build y el número de interfaz son los de la
// versión pública de WoW Forever. Las respuestas van tipadas contra `nucleo.ts`: si cambia un tipo, el escaparate deja
// de compilar. Los nombres de los comandos no están tipados (`main.tsx` falla con el nombre si falta uno).
import type {
  Addon,
  Deteccion,
  EstadoDiagnostico,
  Historial,
  InfoApp,
  Operacion,
  Personaje,
  PiezaPropuesta,
  PropuestaExportacion,
  ResumenPaquete,
  VistaPrevia,
} from "../nucleo";

const RAIZ = String.raw`D:\Games\World of Warcraft`;
const CUENTA = "12345678#1";
// En Forever la carpeta del reino es un número (medido en una instalación real, como en `Personajes.test.tsx`).
const REINO = "93";

export const INFO: InfoApp = {
  nombre: "WoW Profile Manager",
  version: "0.1.0",
  autor: "Sergio Vertedor",
  repositorio: "https://github.com/SergioVertedor/wow-profile-manager",
  koFi: "https://ko-fi.com/tonotabidev",
  web: "https://tonotabi.dev",
  licencia: "https://www.gnu.org/licenses/gpl-3.0.html",
};

function personaje(nombre: string): Personaje {
  return {
    carpetaReino: REINO,
    carpetaPersonaje: nombre,
    nombreEnDatos: { texto: nombre, origen: "datos" },
  };
}

function addon(carpeta: string, titulo: string, version: string, svCuenta: string[] = []): Addon {
  return {
    carpeta,
    toc: `${carpeta}.toc`,
    titulo,
    version,
    enlace: null,
    dependencias: [],
    svCuenta,
    svPersonaje: [],
  };
}

const ADDONS: Addon[] = [
  addon("Bartender4", "Bartender4", "4.15.2", ["Bartender4DB"]),
  addon("Details", "Details! Damage Meter", "#Details.20260412", ["_detalhes_global"]),
  addon("DBM-Core", "Deadly Boss Mods", "11.2.14", ["DBM_AllSavedOptions"]),
  addon("WeakAuras", "WeakAuras", "5.19.9", ["WeakAurasSaved"]),
  addon("Plater", "Plater Nameplates", "v612", ["PlaterDB"]),
  addon("BigWigs", "BigWigs", "v398", ["BigWigs3DB"]),
  addon("OmniCC", "OmniCC", "11.1.4", ["OmniCCDB"]),
  addon("Bagnon", "Bagnon", "11.1.2", ["Bagnon_Sets"]),
  addon("Pawn", "Pawn", "2.11.0", ["PawnCommon"]),
  addon("HandyNotes", "HandyNotes", "v1.6.25", ["HandyNotesDB"]),
  addon("Questie", "Questie", "11.4.0", ["QuestieConfig"]),
  addon("TomTom", "TomTom", "v4.0.9", ["TomTomDB"]),
];

export const PERSONAJES = ["Aelindra", "Brannoc", "Corvina", "Dorrik", "Elowen"];

export const DETECCION: Deteccion = {
  instalaciones: [
    {
      raiz: RAIZ,
      fuente: "battlenet",
      nombre: null,
      versiones: [
        {
          carpeta: "_forever_",
          producto: "wowf",
          soportada: true,
          build: "1.60.1.70959",
          interfaz: 16001,
          cuentas: {
            ignoradas: 0,
            cuentas: [
              {
                carpeta: CUENTA,
                nombre: "Main",
                reinosIgnorados: 0,
                personajes: PERSONAJES.map(personaje),
              },
            ],
          },
          addons: { addons: ADDONS, ignoradas: 0 },
        },
      ],
    },
  ],
  noEncontradas: [],
  preferencias: null,
};

function pieza(
  id: string,
  categoria: PiezaPropuesta["categoria"],
  ambito: PiezaPropuesta["ambito"],
  bytes: number,
  extra: Partial<PiezaPropuesta> = {},
): PiezaPropuesta {
  return {
    id,
    categoria,
    tipo: "fichero",
    ambito,
    bytes,
    quitadas: [],
    marcada: true,
    addon: null,
    variable: null,
    perfil: null,
    regla: null,
    versionNoMedida: false,
    pistas: [],
    ...extra,
  };
}

function configAddon(addonCarpeta: string, variable: string, bytes: number): PiezaPropuesta {
  return pieza(`config-${addonCarpeta}`, "config-addon", "cuenta", bytes, {
    tipo: "perfil-acedb",
    addon: addonCarpeta,
    variable,
    perfil: "Aelindra",
    regla: "perfilAcedb",
  });
}

export const PROPUESTA: PropuestaExportacion = {
  piezas: [
    pieza("macros-personaje", "macros", "personaje", 3_412),
    pieza("macros-cuenta", "macros", "cuenta", 1_208),
    pieza("teclas", "teclas", "personaje", 18_934),
    pieza("teclas-clic", "teclas-clic", "personaje", 412),
    pieza("opciones-personaje", "opciones", "personaje", 6_120, {
      tipo: "fichero-filtrado",
      quitadas: [
        {
          motivo: "estado",
          claves: ["lastAddonVersion", "playerStatsShown", "lastCharacterIndex"],
        },
      ],
    }),
    pieza("chat", "chat", "personaje", 2_048),
    pieza("ventanas", "ventanas", "personaje", 1_536),
    pieza("modo-edicion", "modo-edicion", "cuenta", 2_703, { tipo: "adaptador", perfil: "Healer" }),
    pieza("addons-activos", "addons-activos", "personaje", 742),
    configAddon("Bartender4", "Bartender4DB", 24_310),
    configAddon("Plater", "PlaterDB", 41_872),
    configAddon("OmniCC", "OmniCCDB", 2_904),
  ],
  apartadas: [],
  noExportables: [],
  juegoAbierto: false,
};

const SHA = "9f2c4e1b7a6d3c58e0f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7";
const SHA_PVP = "3b8e0d6a1c2f4b7e9a0d1c3e5f7a9b0c2d4e6f8a1b3c5d7e9f0a2b4c6d8e0f1a";
const SHA_TANQUE = "c71a5e9d3b0f2c4a6e8d1b3f5a7c9e0b2d4f6a8c0e1b3d5f7a9c0e2b4d6f8a1c";

export const PAQUETE: ResumenPaquete = {
  sha256: SHA,
  manifiesto: {
    formato: { nombre: "wowprofile", version: "1.0" },
    perfil: {
      nombre: "Healer setup",
      descripcion: "Bars, keybinds and raid frames for healing.",
      creado: "2026-11-18T20:41:00Z",
    },
    origen: { producto: "wowf", build: "1.60.1.70959", interfaz: 16001 },
    addons: ADDONS.slice(0, 5).map((a) => ({
      id: a.carpeta,
      titulo: a.titulo ?? a.carpeta,
      version: a.version ?? undefined,
      dependencias: [],
    })),
  },
  entradas: [
    {
      id: "macros-personaje",
      categoria: "macros",
      tipo: "fichero",
      ambito: "personaje",
      deCuenta: false,
      bytes: 3_412,
      marcada: true,
      generica: false,
      contenido: {
        tipo: "lista",
        elementos: ["Heal Focus", "Dispel Mouseover", "Mount Up", "Trinket + CD"],
      },
    },
    {
      id: "teclas",
      categoria: "teclas",
      tipo: "fichero",
      ambito: "personaje",
      deCuenta: false,
      bytes: 18_934,
      marcada: true,
      generica: false,
    },
    {
      id: "opciones-personaje",
      categoria: "opciones",
      tipo: "fichero-filtrado",
      ambito: "personaje",
      deCuenta: false,
      bytes: 6_120,
      marcada: true,
      generica: false,
    },
    {
      id: "addons-activos",
      categoria: "addons-activos",
      tipo: "fichero",
      ambito: "personaje",
      deCuenta: false,
      bytes: 742,
      marcada: true,
      generica: false,
      contenido: {
        tipo: "addons",
        activados: ["Bartender4", "Plater", "WeakAuras"],
        desactivados: [],
      },
    },
    {
      id: "config-Bartender4",
      categoria: "config-addon",
      tipo: "perfil-acedb",
      ambito: "cuenta",
      deCuenta: false,
      bytes: 24_310,
      marcada: true,
      generica: false,
      addon: "Bartender4",
      variable: "Bartender4DB",
      perfil: "Healer setup",
    },
  ],
};

export const VISTA_PREVIA: VistaPrevia = {
  plan: {
    personajes: [
      {
        cuenta: CUENTA,
        reino: REINO,
        personaje: "Brannoc",
        conflictos: [],
        conflictosPerfil: [],
        noAplicables: [],
        ficheros: [
          {
            entrada: "macros-personaje",
            entradas: ["macros-personaje"],
            categoria: "macros",
            destino: "personaje/macros-cache.txt",
            deCuenta: false,
            grupo: "modificado",
            accion: "fusionar",
            elementos: [
              { nombre: "Heal Focus", cambio: "anadido" },
              { nombre: "Dispel Mouseover", cambio: "anadido" },
              {
                nombre: "Mount Up",
                cambio: "cambiado",
                antes: "/cast Swift Gryphon",
                despues: "/cast [flyable] Swift Gryphon; Charger",
              },
            ],
            apartadas: [],
            avisos: [],
            bytesAntes: 1_904,
            bytesDespues: 3_412,
          },
          {
            entrada: "teclas",
            entradas: ["teclas"],
            categoria: "teclas",
            destino: "personaje/bindings-cache.wtf",
            deCuenta: false,
            grupo: "nuevo",
            accion: "crear",
            elementos: [],
            apartadas: [],
            avisos: [],
            bytesDespues: 18_934,
          },
          {
            entrada: "addons-activos",
            entradas: ["addons-activos"],
            categoria: "addons-activos",
            destino: "personaje/AddOns.txt",
            deCuenta: false,
            grupo: "modificado",
            accion: "sobrescribir",
            elementos: [
              { nombre: "Plater", cambio: "anadido" },
              { nombre: "WeakAuras", cambio: "anadido" },
            ],
            apartadas: [],
            avisos: [],
            bytesAntes: 610,
            bytesDespues: 742,
          },
          {
            entrada: "config-Bartender4",
            entradas: ["config-Bartender4"],
            categoria: "config-addon",
            destino: "cuenta/SavedVariables/Bartender4.lua",
            deCuenta: false,
            grupo: "modificado",
            accion: "fusionar",
            elementos: [
              { nombre: "Healer setup", cambio: "anadido", clase: "perfil" },
              {
                nombre: "Brannoc",
                cambio: "cambiado",
                clase: "perfilDe",
                antes: "Default",
                despues: "Healer setup",
              },
            ],
            apartadas: [],
            avisos: [],
            bytesAntes: 31_220,
            bytesDespues: 55_530,
          },
        ],
      },
    ],
    incompatibles: [],
  },
  juegoAbierto: false,
  interfazDestino: 16001,
  huella: "escaparate",
};

function operacion(
  id: string,
  tipo: Operacion["tipo"],
  creada: string,
  nombre: string,
  extra: Partial<Operacion> = {},
): Operacion {
  return {
    id,
    tipo,
    creada,
    terminada: creada,
    estado: "completa",
    error: null,
    destino: {
      raizVersion: `${RAIZ}\\_forever_`,
      producto: "wowf",
      cuenta: CUENTA,
      carpetaReino: REINO,
      carpetaPersonaje: nombre,
      nombreEnDatos: nombre,
    },
    paquete: null,
    deshace: null,
    deshechaPor: null,
    deshaceBorrada: false,
    ficheros: 4,
    bytes: 82_618,
    deshacible: true,
    aMedias: false,
    porResolver: false,
    sinDevolver: [],
    ...extra,
  };
}

export const HISTORIAL: Historial = {
  operaciones: [
    operacion("20261119-184210-a1", "importacion", "2026-11-19T18:42:10Z", "Brannoc", {
      paquete: { nombre: "Healer setup", sha256: SHA },
    }),
    operacion("20261118-213005-b2", "deshacer", "2026-11-18T21:30:05Z", "Corvina", {
      deshace: "20261118-212840-c3",
      deshacible: false,
      ficheros: 3,
      bytes: 24_904,
    }),
    operacion("20261118-212840-c3", "importacion", "2026-11-18T21:28:40Z", "Corvina", {
      estado: "deshecha",
      paquete: { nombre: "PvP arena binds", sha256: SHA_PVP },
      deshechaPor: "20261118-213005-b2",
      deshacible: false,
      ficheros: 3,
      bytes: 24_904,
    }),
    operacion("20261117-101512-d4", "importacion", "2026-11-17T10:15:12Z", "Dorrik", {
      paquete: { nombre: "Tank essentials", sha256: SHA_TANQUE },
      ficheros: 6,
      bytes: 120_455,
    }),
  ],
  bytes: 252_881,
  ilegibles: [],
};

export const DIAGNOSTICO: EstadoDiagnostico = {
  guarda: false,
  maximo: 2000,
  lineas: [
    {
      n: 1,
      instante: "2026-11-19T18:40:02Z",
      nivel: "INFO",
      modulo: "wow_profile_manager",
      mensaje: "WoW Profile Manager 0.1.0 starting",
    },
    {
      n: 2,
      instante: "2026-11-19T18:40:02Z",
      nivel: "INFO",
      modulo: "deteccion",
      mensaje: `installation found via Battle.net: ${RAIZ}`,
    },
    {
      n: 3,
      instante: "2026-11-19T18:40:03Z",
      nivel: "INFO",
      modulo: "descubrimiento",
      mensaje: "_forever_: 1 account, 5 characters, 12 addons",
    },
  ],
};
