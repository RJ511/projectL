import {
  FolderOpen,
  FilePlus,
  Focus,
  Calendar,
  Brain,
  SlidersHorizontal,
} from "lucide-react";

export default function ToolBar({
  onOpenOlm,
  isOlmOpen = false,
  onOpenPlanner,
  isPlannerOpen = false,
  onOpenSettings,
  isSettingsOpen = false,
  onCreateQuizTemplate,
}) {
  const icons = [
    {
      icon: <Brain size={18} />,
      label: "OLM",
      onClick: onOpenOlm,
      active: isOlmOpen,
    },
    { icon: <FolderOpen size={20} />, label: "Explorador" },
    {
      icon: <FilePlus size={20} />,
      label: "Novo quiz/teste",
      onClick: onCreateQuizTemplate,
    },
    { icon: <Focus size={20} />, label: "Concentração" },
    {
      icon: <Calendar size={20} />,
      label: "Calendário",
      onClick: onOpenPlanner,
      active: isPlannerOpen,
    },
    {
      icon: <SlidersHorizontal size={18} />,
      label: "Configurações",
      onClick: onOpenSettings,
      active: isSettingsOpen,
    },
  ];

  return (
    <div
      style={{
        width: 48,
        minWidth: 48,
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: 12,
        padding: "12px 6px",
        borderRight: "1px solid #dbe5f4",
        background: "#ffffff",
        boxShadow: "inset -1px 0 0 #eef2ff",
      }}
    >
      {icons.map((item, i) => (
        <button
          key={i}
          type="button"
          onClick={item.onClick}
          style={{
            width: 34,
            height: 34,
            borderRadius: 10,
            border: "1px solid #dbe5f4",
            background: item.active ? "#dbeafe" : "#f8fbff",
            boxShadow: "none",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: item.active ? "#1d4ed8" : "#334155",
            padding: 0,
          }}
          title={item.label}
        >
          {item.icon}
        </button>
      ))}
    </div>
  );
}
