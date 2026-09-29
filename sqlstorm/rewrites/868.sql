
WITH UserStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        SUM(b.Class) AS TotalBadgeCount,
        COUNT(p.Id) AS TotalPosts,
        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
        MAX(p.CreationDate) AS LastPostDate
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    GROUP BY 
        u.Id, u.DisplayName
),
RecentPostStats AS (
    SELECT 
        p.OwnerUserId,
        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(*) / 1e6 AS AvgActivityDuration
    FROM
        Posts p
    WHERE 
        p.LastActivityDate IS NOT NULL
    GROUP BY 
        p.OwnerUserId
),
PostVoteStats AS (
    SELECT 
        p.OwnerUserId,
        COUNT(v.Id) AS TotalVotes,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
    FROM 
        Posts p
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    GROUP BY 
        p.OwnerUserId
)
SELECT 
    us.UserId,
    us.DisplayName,
    us.TotalBadgeCount,
    us.TotalPosts,
    us.QuestionCount,
    us.AnswerCount,
    COALESCE(rps.AvgActivityDuration, 0) AS AvgActivityDuration,
    COALESCE(pvs.TotalVotes, 0) AS TotalVotes,
    COALESCE(pvs.UpVotes, 0) AS UpVotes,
    COALESCE(pvs.DownVotes, 0) AS DownVotes,
    RANK() OVER (ORDER BY us.TotalPosts DESC) AS UserRank
FROM 
    UserStats us
LEFT JOIN 
    RecentPostStats rps ON us.UserId = rps.OwnerUserId
LEFT JOIN 
    PostVoteStats pvs ON us.UserId = pvs.OwnerUserId
WHERE 
    us.TotalPosts > 10
ORDER BY 
    UserRank, us.DisplayName;
