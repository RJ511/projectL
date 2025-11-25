import Sidebar from "./Sidebar";

const mockTree = [
  {
    name: "docs",
    path: "/docs",
    is_dir: true,
    children: [
      {
        name: "intro.md",
        path: "/docs/intro.md",
        is_dir: false,
      },
      {
        name: "setup.md",
        path: "/docs/setup.md",
        is_dir: false,
      },
    ],
  },
  {
    name: "README.md",
    path: "/README.md",
    is_dir: false,
  },
];

export default {
  title: "Layout/Sidebar",
  component: Sidebar,
};

export const Default = {
  args: {
    tree: mockTree,
    chooseDirectory: () => alert("Escolher pasta (mock)"),
    createMarkdown: () => alert("Novo .md (mock)"),
    openFile: (file) => alert("Abrir ficheiro: " + JSON.stringify(file)),
  },
};
