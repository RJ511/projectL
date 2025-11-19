import { renderTree } from "../fileTree/RenderTree";


export default function Sidebar({ tree = [], chooseDirectory, createMarkdown, openFile }) {
  return (
    <div style={{ width: 250, padding: 10, borderRight: "1px solid #ddd" }}>
      <button onClick={chooseDirectory}>Escolher Pasta</button>
      <button onClick={createMarkdown}>Novo .md</button>
      <button
        onClick={() => {
          localStorage.removeItem("lastRootPath");
          chooseDirectory();
        }}
      >
        Mudar Pasta
      </button>


      <h3>Ficheiros</h3>
      {tree.length === 0 ? <p>Nenhuma pasta</p> : renderTree(tree, openFile)}
    </div>
  );
}
