<script setup lang="ts">
import { ref, watch, onUnmounted, nextTick, computed, onMounted } from 'vue';
import gsap from 'gsap';

interface Props {
  isOpen: boolean;
  layoutId?: string;
  anchorEl?: HTMLElement | null;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'close'): void;
}>();

const modalRef = ref<HTMLDivElement | null>(null);
const positionStyle = ref<Record<string, string>>({});
const isAnchored = computed(() => !!props.anchorEl);

const updatePosition = () => {
  if (!props.isOpen || !modalRef.value) return;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const isMobile = vw < 768;

  if (isMobile) {
    positionStyle.value = {
      position: 'fixed',
      top: '0',
      left: '0',
      width: '100%',
      height: '100%',
      maxHeight: '100vh',
      margin: '0',
      borderRadius: '0',
      overflowY: 'auto',
    };
    return;
  }

  const anchorRect = props.anchorEl?.getBoundingClientRect();

  if (!anchorRect) {
    positionStyle.value = {
      inset: '0',
      width: 'fit-content',
      height: 'fit-content',
      margin: 'auto',
      maxHeight: '90vh',
      borderRadius: '12px',
      overflowY: 'auto',
    };
    return;
  }

  const PAD = 12;
  
  modalRef.value.style.width = '';
  modalRef.value.style.height = '';
  
  const modalW = modalRef.value.offsetWidth || 420;
  const modalH = modalRef.value.offsetHeight || 520;

  const { top, left, width, height, bottom } = anchorRect;

  const centerX = left + width / 2;
  const centerY = top + height / 2;

  let posX = centerX - modalW / 2;
  const TOP_PAD = 40 + 12;
  const BOTTOM_PAD = 12;

  let posY = centerY - modalH / 2;
  
  if (posY < TOP_PAD) {
    posY = Math.max(TOP_PAD, top - 20); 
  } 
  else if (posY + modalH > vh - BOTTOM_PAD) {
    posY = Math.min(vh - modalH - BOTTOM_PAD, bottom + 20 - modalH);
  }

  posX = Math.max(PAD, Math.min(posX, vw - modalW - PAD));
  posY = Math.max(TOP_PAD, Math.min(posY, vh - modalH - BOTTOM_PAD));

  positionStyle.value = {
    left: `${posX}px`,
    top: `${posY}px`,
    width: `${modalW}px`,
    height: `${modalH}px`,
    maxHeight: `calc(100vh - ${PAD * 2}px)`,
    borderRadius: '12px',
    overflowY: "auto",
  };
};

watch(() => props.anchorEl, () => {
    if (props.isOpen) {
        nextTick(updatePosition);
    }
});

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === "Escape") emit('close');
};

let prevOverflow = '';
let prevPaddingRight = '';

watch(() => props.isOpen, (open) => {
  if (open) {
    window.addEventListener("keydown", handleKeyDown);
    const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;
    prevOverflow = document.body.style.overflow;
    prevPaddingRight = document.body.style.paddingRight;
    document.body.style.overflow = "hidden";
    if (scrollbarWidth > 0) {
      document.body.style.paddingRight = `${scrollbarWidth}px`;
    }
  } else {
    window.removeEventListener("keydown", handleKeyDown);
    document.body.style.overflow = prevOverflow;
    document.body.style.paddingRight = prevPaddingRight;
  }
});

onMounted(() => {
  window.addEventListener("resize", updatePosition);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleKeyDown);
  window.removeEventListener("resize", updatePosition);
  document.body.style.overflow = prevOverflow;
  document.body.style.paddingRight = prevPaddingRight;
});

const onEnter = async (el: Element, done: () => void) => {
  const target = el as HTMLElement;
  
  updatePosition();
  await nextTick();
  
  const anchorRect = props.anchorEl?.getBoundingClientRect();

  if (anchorRect && props.layoutId) {
    const finalRect = target.getBoundingClientRect();
    
    const scaleX = anchorRect.width / finalRect.width;
    const scaleY = anchorRect.height / finalRect.height;
    
    const anchorCenterX = anchorRect.left + anchorRect.width / 2;
    const anchorCenterY = anchorRect.top + anchorRect.height / 2;
    const targetCenterX = finalRect.left + finalRect.width / 2;
    const targetCenterY = finalRect.top + finalRect.height / 2;

    const deltaX = anchorCenterX - targetCenterX;
    const deltaY = anchorCenterY - targetCenterY;

    const anchorBg = window.getComputedStyle(props.anchorEl!).backgroundColor;
    const targetBg = window.getComputedStyle(target).backgroundColor;

    gsap.fromTo(target, 
      { 
        x: deltaX, 
        y: deltaY, 
        scaleX: scaleX, 
        scaleY: scaleY,
        opacity: 1,
        backgroundColor: anchorBg,
        transformOrigin: "center center", 
        borderRadius: "12px",
      }, 
      {
        x: 0,
        y: 0,
        scaleX: 1,
        scaleY: 1,
        opacity: 1,
        backgroundColor: targetBg,
        borderRadius: "12px",
        duration: 0.3,
        ease: "power2.out",
        onComplete: () => {
          gsap.set(target, { clearProps: "transform,transformOrigin,background-color" });
          done();
        }
      }
    );
  } else {
    gsap.fromTo(target, 
      { scale: 0.95, opacity: 0 }, 
      { scale: 1, opacity: 1, duration: 0., ease: "back.out(1.5)", onComplete: done }
    );
  }
};

const onLeave = (el: Element, done: () => void) => {
  const target = el as HTMLElement;
  const anchorRect = props.anchorEl?.getBoundingClientRect();

  if (anchorRect && props.layoutId) {
    const finalRect = target.getBoundingClientRect();
    
    const scaleX = anchorRect.width / finalRect.width;
    const scaleY = anchorRect.height / finalRect.height;
    
    const anchorCenterX = anchorRect.left + anchorRect.width / 2;
    const anchorCenterY = anchorRect.top + anchorRect.height / 2;
    const targetCenterX = finalRect.left + finalRect.width / 2;
    const targetCenterY = finalRect.top + finalRect.height / 2;

    const deltaX = anchorCenterX - targetCenterX;
    const deltaY = anchorCenterY - targetCenterY;

    const anchorBg = window.getComputedStyle(props.anchorEl!).backgroundColor;

    gsap.to(target, {
      x: deltaX,
      y: deltaY,
      scaleX: scaleX,
      scaleY: scaleY,
      backgroundColor: anchorBg,
      opacity: 1,
      duration: 0.3,
      borderRadius: "12px",
      transformOrigin: "center center",
      ease: "power2.in",
      onComplete: done
    });
  } else {
    gsap.to(target, { 
      scale: 0.95, opacity: 0, duration: 0.2, ease: "power2.in", onComplete: done 
    });
  }
};
</script>

<script lang="ts">
export default {
  inheritAttrs: false
}
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-300"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-300"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div 
        v-if="isOpen" 
        :class="[
          'fixed inset-0 z-40', 
          isAnchored ? 'bg-transparent' : 'bg-black/10 dark:bg-black/20 backdrop-blur-[2px]',
          $attrs.class
        ]"
        @click="emit('close')"
      />
    </Transition>

    <Transition 
      @enter="onEnter" 
      @leave="onLeave" 
      :css="false"
    >
      <div
        v-if="isOpen"
        ref="modalRef"
        v-bind="$attrs"
        :style="positionStyle"
        :class="[
          'absolute z-50 bg-white dark:bg-[#181818] shadow-2xl overflow-hidden', 
          !isAnchored ? 'w-full md:min-w-[400px] md:w-auto' : 'w-full md:min-w-[192px] md:w-auto',
          $attrs.class
        ]"
      >
        <slot />
      </div>
    </Transition>
  </Teleport>
</template>