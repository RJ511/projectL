import { FolderOpen, FilePlus, Focus, Calendar } from "lucide-react";

export default function ToolBar() {
  const icons = [
    { icon: <FolderOpen size={20} />, label: "Explorador" },
    { icon: <FilePlus size={20} />, label: "Novo ficheiro" },
    { icon: <Focus size={20} />, label: "Concentração" },
    { icon: <Calendar size={20} />, label: "Calendário" },
  ];

  return (
    <div className="h-full w-[3%] min-w-[40px] flex flex-col items-center gap-4 py-4 border-r bg-neutral-50">
      {icons.map((item, i) => (
        <button
          key={i}
          className="p-2 hover:bg-neutral-200 rounded-xl"
          title={item.label}
        >
          {item.icon}
        </button>
      ))}
    </div>
  );
}
