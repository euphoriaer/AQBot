import type { Conversation, ConversationCategory } from '@/types'

export const NATIVE_LIST_MAX_ROWS = 159
export const VIRTUAL_LIST_MIN_ROWS = 160
export const SIDEBAR_OVERSCAN = 8

export type ConversationListRow =
  | {
    type: 'groupHeader'
    key: string
    group: string
    category: ConversationCategory | null
    collapsible: boolean
    expanded: boolean
  }
  | {
    type: 'conversation'
    key: string
    group: string
    conversation: Conversation
    isChild: boolean
    isPinnedShortcut: boolean
    childCount: number
    expanded: boolean
  }
  | {
    type: 'emptyCategory'
    key: string
    group: string
    category: ConversationCategory
  }

interface BuildConversationRowsInput {
  conversations: readonly Conversation[]
  categories: readonly ConversationCategory[]
  expandedParentIds: ReadonlySet<string>
  expandedGroupKeys: ReadonlySet<string>
  nowSeconds?: number
}

export function filterConversationsWithParents(
  conversations: readonly Conversation[],
  rawQuery: string,
): readonly Conversation[] {
  const query = rawQuery.trim().toLocaleLowerCase()
  if (!query) return conversations

  const conversationById = new Map(conversations.map((item) => [item.id, item]))
  const includedIds = new Set<string>()
  for (const conversation of conversations) {
    if (!conversation.title.toLocaleLowerCase().includes(query)) continue
    includedIds.add(conversation.id)

    let parentId = conversation.parent_conversation_id
    while (parentId && !includedIds.has(parentId)) {
      includedIds.add(parentId)
      parentId = conversationById.get(parentId)?.parent_conversation_id ?? null
    }
  }

  return conversations.filter((conversation) => includedIds.has(conversation.id))
}

export function getSearchExpandedParentIds(
  conversations: readonly Conversation[],
  rawQuery: string,
): ReadonlySet<string> {
  const query = rawQuery.trim().toLocaleLowerCase()
  const expandedParentIds = new Set<string>()
  if (!query) return expandedParentIds

  const conversationById = new Map(conversations.map((item) => [item.id, item]))
  for (const conversation of conversations) {
    if (!conversation.title.toLocaleLowerCase().includes(query)) continue

    let parentId = conversation.parent_conversation_id
    while (parentId && !expandedParentIds.has(parentId)) {
      expandedParentIds.add(parentId)
      parentId = conversationById.get(parentId)?.parent_conversation_id ?? null
    }
  }

  return expandedParentIds
}

function sortConversations(items: readonly Conversation[]): Conversation[] {
  return [...items].sort((a, b) => {
    if (a.sort_order !== b.sort_order) return a.sort_order - b.sort_order
    return b.updated_at - a.updated_at
  })
}

export function buildConversationRows({
  conversations,
  categories,
  expandedParentIds,
  expandedGroupKeys,
}: BuildConversationRowsInput): ConversationListRow[] {
  const childrenByParent = new Map<string, Conversation[]>()
  const topLevel: Conversation[] = []
  for (const conversation of conversations) {
    if (conversation.parent_conversation_id) {
      const children = childrenByParent.get(conversation.parent_conversation_id)
      if (children) children.push(conversation)
      else childrenByParent.set(conversation.parent_conversation_id, [conversation])
    } else {
      topLevel.push(conversation)
    }
  }

  const conversationsByCategory = new Map<string, Conversation[]>()
  const uncategorized: Conversation[] = []
  for (const conversation of topLevel) {
    if (conversation.category_id) {
      const grouped = conversationsByCategory.get(conversation.category_id)
      if (grouped) grouped.push(conversation)
      else conversationsByCategory.set(conversation.category_id, [conversation])
    } else {
      uncategorized.push(conversation)
    }
  }

  const rows: ConversationListRow[] = []

  const pushConversationRecursive = (
    conversation: Conversation,
    group: string,
    isChild: boolean,
  ) => {
    const children = childrenByParent.get(conversation.id) ?? []
    rows.push({
      type: 'conversation',
      key: `conversation:${conversation.id}`,
      group,
      conversation,
      isChild,
      isPinnedShortcut: false,
      childCount: children.length,
      expanded: expandedParentIds.has(conversation.id),
    })
    if (!expandedParentIds.has(conversation.id)) return
    for (const child of sortConversations(children)) {
      pushConversationRecursive(child, group, true)
    }
  }

  const pinnedTopLevel = topLevel.filter((c) => c.is_pinned)
  if (pinnedTopLevel.length > 0) {
    const group = 'pinned'
    rows.push({
      type: 'groupHeader',
      key: `group:${group}`,
      group,
      category: null,
      collapsible: false,
      expanded: true,
    })
    for (const conversation of sortConversations(pinnedTopLevel)) {
      rows.push({
        type: 'conversation',
        key: `pinned-shortcut:${conversation.id}`,
        group,
        conversation,
        isChild: false,
        isPinnedShortcut: true,
        childCount: 0,
        expanded: false,
      })
    }
  }

  for (const category of categories) {
    const group = `cat:${category.id}`
    const expanded = expandedGroupKeys.has(group)
    rows.push({
      type: 'groupHeader',
      key: `group:${group}`,
      group,
      category,
      collapsible: true,
      expanded,
    })
    if (!expanded) continue

    const grouped = conversationsByCategory.get(category.id)
    if (grouped?.length) {
      for (const conversation of sortConversations(grouped)) {
        pushConversationRecursive(conversation, group, false)
      }
    } else {
      rows.push({
        type: 'emptyCategory',
        key: `empty:${category.id}`,
        group,
        category,
      })
    }
  }

  if (uncategorized.length > 0) {
    const group = 'uncategorized'
    rows.push({
      type: 'groupHeader',
      key: `group:${group}`,
      group,
      category: null,
      collapsible: false,
      expanded: true,
    })
    for (const conversation of sortConversations(uncategorized)) {
      pushConversationRecursive(conversation, group, false)
    }
  }

  return rows
}
