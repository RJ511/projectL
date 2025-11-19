import requests
import json
from urllib.parse import urlencode

# ------------------------------------------
# CONFIGURAÇÃO
# ------------------------------------------

KEYWORDS = [
    "self-regulated learning",
    "open learner model",
    "learning analytics dashboards",
    "self-directed learning",
    "personal learning environment",
    "meta-learning",
    "habit tracking learning"
]

MAX_RESULTS = 100

# ------------------------------------------
# FUNÇÃO: Buscar no CrossRef
# ------------------------------------------
def search_crossref(query):
    url = "https://api.crossref.org/works?" + urlencode({
        "query": query,
        "rows": MAX_RESULTS
    })

    r = requests.get(url)
    data = r.json()

    results = []
    for item in data["message"]["items"]:
        results.append({
            "title": item.get("title", [""])[0],
            "authors": item.get("author", []),
            "year": item.get("published-print", {"date-parts":[["?"]]} )["date-parts"][0][0],
            "doi": item.get("DOI", None),
            "url": item.get("URL", None),
            "source": "CrossRef"
        })

    return results

# ------------------------------------------
# FUNÇÃO: Buscar no Semantic Scholar
# ------------------------------------------
def search_semantic_scholar(query):
    url = "https://api.semanticscholar.org/graph/v1/paper/search?" + urlencode({
        "query": query,
        "limit": MAX_RESULTS,
        "fields": "title,year,authors,url,doi"
    })

    r = requests.get(url)
    data = r.json()

    results = []
    for item in data.get("data", []):
        results.append({
            "title": item.get("title", ""),
            "authors": item.get("authors", []),
            "year": item.get("year", ""),
            "doi": item.get("doi", None),
            "url": item.get("url", None),
            "source": "Semantic Scholar"
        })

    return results


# ------------------------------------------
# FUNÇÃO: Buscar no arXiv
# ------------------------------------------
def search_arxiv(query):
    url = "http://export.arxiv.org/api/query?" + urlencode({
        "search_query": query,
        "start": 0,
        "max_results": MAX_RESULTS
    })

    r = requests.get(url)

    # resposta XML -> texto simples
    entries = r.text.split("<entry>")[1:]

    results = []
    for entry in entries:
        try:
            title = entry.split("<title>")[1].split("</title>")[0].strip()
            link = entry.split("<id>")[1].split("</id>")[0].strip()
            results.append({
                "title": title,
                "url": link,
                "year": "arXiv",
                "source": "arXiv"
            })
        except:
            continue

    return results

# ------------------------------------------
# MAIN
# ------------------------------------------
if __name__ == "__main__":
    all_results = []

    for kw in KEYWORDS:
        print(f"\n🔍 Searching for: {kw}")

        cr = search_crossref(kw)
        ss = search_semantic_scholar(kw)
        ax = search_arxiv(kw)

        combined = cr + ss + ax
        all_results.extend(combined)

        for r in combined:
            print(f" - {r['title']} ({r['source']})")
            if r.get("url"):
                print(f"   {r['url']}")
        print("\n" + "-"*50)

    # Guardar em JSON
    with open("article_results.json", "w", encoding="utf-8") as f:
        json.dump(all_results, f, indent=4, ensure_ascii=False)

    print("\n✔️ Artigos guardados em article_results.json")
