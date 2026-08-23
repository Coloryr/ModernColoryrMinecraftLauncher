<script setup lang="ts">
// 账户平铺视图：皮肤 / 头像 / 披风 三图卡片
import { t } from "../../../lib/i18n";
import { avatarImage, capeImage, skinImage } from "../../../lib/accountImages";
import AccountActions from "../../../components/AccountActions.vue";
import type { Account } from "../../../lib/types";

const props = defineProps<{
  accounts: Account[];
  currentUuid: string;
  typeLabel: (acc: Account) => string;
  seedOf: (uuid: string) => number;
}>();

const emit = defineEmits<{
  (e: "switch", acc: Account): void;
  (e: "refresh", acc: Account): void;
  (e: "relogin", acc: Account): void;
  (e: "delete", acc: Account): void;
}>();

function isCurrent(acc: Account): boolean {
  return acc.uuid === props.currentUuid;
}
</script>

<template>
  <div class="acc-grid">
    <div
      v-for="acc in accounts"
      :key="acc.uuid"
      class="acc-card"
      :class="{ current: isCurrent(acc) }"
      @dblclick="emit('switch', acc)"
    >
      <div class="acc-images">
        <span v-if="isCurrent(acc)" class="current-badge">{{ t("account.current") }}</span>
        <img :src="avatarImage(seedOf(acc.uuid), acc.skin)" class="img-avatar" :alt="t('account.avatar')" />
        <img :src="skinImage(seedOf(acc.uuid), acc.skin)" class="img-skin" :alt="t('account.skin')" />
        <img :src="capeImage(seedOf(acc.uuid), acc.skin)" class="img-cape" :alt="t('account.cape')" />
      </div>
      <div class="acc-head">
        <span class="acc-name">{{ acc.name }}</span>
        <span class="acc-type" :class="acc.type">{{ typeLabel(acc) }}</span>
        <AccountActions
          @refresh="emit('refresh', acc)"
          @relogin="emit('relogin', acc)"
          @delete="emit('delete', acc)"
        />
      </div>
    </div>
    <div v-if="accounts.length === 0" class="empty-tip">{{ t("account.searchEmpty") }}</div>
  </div>
</template>

<style scoped>
.acc-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

/* 空状态：横跨全部列并在内容区居中 */
.empty-tip {
  grid-column: 1 / -1;
  padding: 48px 0;
}

.acc-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.acc-images {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 12px;
  justify-content: center;
  padding: 18px 0;
  background: var(--bg-side);
  border-radius: 12px;
}

.current-badge {
  position: absolute;
  top: 8px;
  left: 8px;
  font-size: 10.5px;
  padding: 2px 9px;
  border-radius: 20px;
  background: var(--accent);
  color: #fff;
  white-space: nowrap;
}

.img-avatar {
  width: 72px;
  height: 72px;
  border-radius: 8px;
  image-rendering: pixelated;
}

.img-skin {
  width: 36px;
  height: 72px;
  border-radius: 5px;
  image-rendering: pixelated;
}

.img-cape {
  width: 72px;
  height: 36px;
  border-radius: 5px;
  image-rendering: pixelated;
}

.acc-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.acc-name {
  flex: 1;
  min-width: 0;
  font-size: 14px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.acc-card.current {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.acc-type {
  font-size: 10.5px;
  padding: 2px 8px;
  border-radius: 20px;
  white-space: nowrap;
  background: var(--bg-hover);
  color: var(--text-dim);
  flex-shrink: 0;
}

.acc-type.microsoft {
  background: rgba(63, 140, 255, 0.16);
  color: #8fb0ff;
}

.acc-type.littleskin {
  background: rgba(168, 85, 247, 0.16);
  color: #c084fc;
}

.acc-type.authlib {
  background: rgba(245, 158, 11, 0.16);
  color: var(--yellow);
}

.acc-type.nide8 {
  background: rgba(6, 182, 212, 0.16);
  color: #22d3ee;
}
</style>
