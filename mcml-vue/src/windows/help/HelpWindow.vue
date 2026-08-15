<script setup lang="ts">
import { computed, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import { t, locale } from "../../lib/i18n";
import { faqs as zhFaqs } from "../../lib/i18n/locales/zh-CN";
import { faqs as enFaqs } from "../../lib/i18n/locales/en-US";

const faqs = computed(() => (locale.value === "en-US" ? enFaqs : zhFaqs));

const openIndex = ref<number | null>(0);

function toggle(i: number) {
  openIndex.value = openIndex.value === i ? null : i;
}
</script>

<template>
  <WindowFrame :title="t('features.help')" @close="$emit('close')">
    <div class="help-list">
      <div v-for="(item, i) in faqs" :key="i" class="faq-item">
        <button class="faq-q" @click="toggle(i)">
          <span>{{ item.q }}</span>
          <svg
            viewBox="0 0 24 24"
            width="14"
            height="14"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            :style="{ transform: openIndex === i ? 'rotate(180deg)' : 'none' }"
          >
            <path d="m6 9 6 6 6-6" />
          </svg>
        </button>
        <div v-if="openIndex === i" class="faq-a">{{ item.a }}</div>
      </div>
    </div>

    <div class="footer-note">
      <p>{{ t("winHelp.stillStuck") }}</p>
      <p>{{ t("winHelp.report") }}</p>
    </div>
  </WindowFrame>
</template>

<style scoped>
.help-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.faq-item {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.faq-q {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 15px 18px;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 14px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
}

.faq-q svg {
  color: var(--text-dim);
  transition: transform 0.15s;
  flex-shrink: 0;
}

.faq-q:hover {
  background: var(--bg-hover);
}

.faq-a {
  padding: 4px 18px 16px;
  font-size: 13px;
  color: var(--text-dim);
  line-height: 1.7;
}

.footer-note {
  margin-top: 22px;
  padding: 16px 18px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  font-size: 12.5px;
  color: var(--text-dim);
  line-height: 1.8;
}
</style>
