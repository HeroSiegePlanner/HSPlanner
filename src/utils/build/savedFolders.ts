import {
  type Folder,
  newId,
  readLibrary,
  writeLibrary,
} from './savedBuilds'

export function listFolders(): Folder[] {
  return readLibrary().folders.slice()
}

export function createFolder(name: string, parentId: string | null): Folder {
  const library = readLibrary()
  const validParent =
    parentId !== null && library.folders.some((f) => f.id === parentId)
      ? parentId
      : null
  const folder: Folder = {
    id: newId('f'),
    name,
    parentId: validParent,
    createdAt: new Date().toISOString(),
  }
  library.folders.push(folder)
  writeLibrary(library)
  return folder
}

export function renameFolder(folderId: string, name: string): Folder | null {
  const library = readLibrary()
  const folder = library.folders.find((f) => f.id === folderId)
  if (!folder) return null
  folder.name = name
  writeLibrary(library)
  return folder
}

function collectDescendants(
  rootId: string,
  folders: Folder[],
): Set<string> {
  const out = new Set<string>([rootId])
  let added = true
  while (added) {
    added = false
    for (const f of folders) {
      if (f.parentId !== null && out.has(f.parentId) && !out.has(f.id)) {
        out.add(f.id)
        added = true
      }
    }
  }
  return out
}

export function deleteFolder(
  folderId: string,
  opts: { cascade: boolean },
): void {
  const library = readLibrary()
  const folder = library.folders.find((f) => f.id === folderId)
  if (!folder) return

  if (opts.cascade) {
    const toDelete = collectDescendants(folderId, library.folders)
    library.folders = library.folders.filter((f) => !toDelete.has(f.id))
    library.builds = library.builds.filter(
      (b) => b.folderId === null || !toDelete.has(b.folderId),
    )
  } else {
    for (const f of library.folders) {
      if (f.parentId === folderId) f.parentId = folder.parentId
    }
    for (const b of library.builds) {
      if (b.folderId === folderId) b.folderId = null
    }
    library.folders = library.folders.filter((f) => f.id !== folderId)
  }

  writeLibrary(library)
}
