class BackgroundVideoManager {
    constructor(cycleInterval = 10000) {
        this.videos = Array.from(document.querySelectorAll('.bg-video'));
        // Start with a random video
        this.currentIndex = Math.floor(Math.random() * this.videos.length);
        this.cycleInterval = cycleInterval;
        
        // Switch to random initial video and start cycling if multiple videos exist
        if (this.videos.length > 1) {
            this.switchToVideo(this.currentIndex);
            this.startCycle();
        }
    }

    switchToVideo(index) {
        // Remove active class from all videos
        this.videos.forEach(video => {
            video.classList.remove('active');
            video.pause(); // Pause inactive videos to save resources
        });

        // Add active class to new video
        const nextVideo = this.videos[index];
        nextVideo.classList.add('active');
        nextVideo.play();
    }

    startCycle() {
        setInterval(() => {
            this.currentIndex = (this.currentIndex + 1) % this.videos.length;
            this.switchToVideo(this.currentIndex);
        }, this.cycleInterval);
    }
}

// Initialize when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new BackgroundVideoManager();
}); 