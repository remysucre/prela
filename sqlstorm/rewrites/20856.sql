
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.Score,
        p.OwnerUserId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS RankByScore,
        COUNT(*) OVER (PARTITION BY p.OwnerUserId) AS TotalPosts
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),
UserVoteStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        SUM(CASE WHEN v.VoteTypeId = 4 THEN 1 ELSE 0 END) AS OffensiveVotes
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    GROUP BY 
        u.Id, u.DisplayName
),
OutstandingBadges AS (
    SELECT 
        b.UserId,
        b.Name,
        COUNT(b.Id) AS BadgeCount
    FROM 
        Badges b
    WHERE 
        b.Class IN (1, 2)  
    GROUP BY 
        b.UserId, b.Name
),
PostHistoryDetails AS (
    SELECT 
        ph.PostId,
        STRING_AGG(CONCAT(ph.UserDisplayName, ' (', ph.CreationDate, ') - ', ph.Comment), ' | ' ORDER BY ph.Id) AS EditHistory
    FROM 
        PostHistory ph
    WHERE 
        ph.PostHistoryTypeId IN (4, 5) 
    GROUP BY 
        ph.PostId
)
SELECT 
    u.DisplayName,
    COUNT(DISTINCT rp.PostId) AS NumberOfPosts,
    COALESCE(SUM(CASE WHEN uvs.UpVotes > uvs.DownVotes THEN uvs.UpVotes ELSE 0 END), 0) AS NetUpvoteScore,
    COALESCE(SUM(CASE WHEN uvs.UpVotes < uvs.DownVotes THEN uvs.DownVotes ELSE 0 END), 0) AS NetDownvoteScore,
    ob.BadgeCount,
    pvd.EditHistory
FROM 
    Users u
LEFT JOIN 
    RankedPosts rp ON u.Id = rp.OwnerUserId
LEFT JOIN 
    UserVoteStats uvs ON u.Id = uvs.UserId
LEFT JOIN 
    OutstandingBadges ob ON u.Id = ob.UserId
LEFT JOIN 
    PostHistoryDetails pvd ON rp.PostId = pvd.PostId
WHERE 
    rp.RankByScore = 1 OR rp.TotalPosts > 3   
GROUP BY 
    u.Id, u.DisplayName, ob.BadgeCount, pvd.EditHistory
HAVING 
    COUNT(DISTINCT rp.PostId) > 0 
ORDER BY 
    NumberOfPosts DESC, NetUpvoteScore DESC, u.Id, ob.BadgeCount, pvd.EditHistory
LIMIT 50;
