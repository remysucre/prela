
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.OwnerUserId,
        p.Score,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS Rank,
        COUNT(DISTINCT c.Id) AS CommentCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVoteCount,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVoteCount
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    GROUP BY 
        p.Id, p.Title, p.CreationDate, p.OwnerUserId, p.Score
),
RecentBadges AS (
    SELECT 
        u.DisplayName,
        b.Name AS BadgeName,
        b.Date AS BadgeDate
    FROM 
        Users u
    JOIN 
        Badges b ON u.Id = b.UserId
    WHERE 
        b.Date >= CURRENT_TIMESTAMP - INTERVAL '30 days'
),
PostHistoryAggregates AS (
    SELECT 
        ph.PostId,
        COUNT(CASE WHEN ph.PostHistoryTypeId IN (10, 11) THEN 1 END) AS CloseVotes,
        COUNT(CASE WHEN ph.PostHistoryTypeId IN (12, 13) THEN 1 END) AS DeleteVotes,
        COUNT(*) AS TotalHistoryCount
    FROM 
        PostHistory ph
    GROUP BY 
        ph.PostId
)
SELECT 
    rp.PostId,
    rp.Title,
    rp.CreationDate,
    rp.Score,
    rp.CommentCount,
    rb.DisplayName AS BadgeOwner,
    rb.BadgeName,
    pha.CloseVotes,
    pha.DeleteVotes,
    (rp.UpVoteCount - rp.DownVoteCount) AS NetVotes,
    CASE 
        WHEN pha.TotalHistoryCount > 0 THEN 'Has History'
        ELSE 'No History'
    END AS HistoryStatus
FROM 
    RankedPosts rp
LEFT JOIN 
    RecentBadges rb ON rp.OwnerUserId = (SELECT u.Id FROM Users u WHERE u.DisplayName = rb.DisplayName LIMIT 1)
LEFT JOIN 
    PostHistoryAggregates pha ON rp.PostId = pha.PostId
WHERE 
    rp.Rank = 1
AND 
    rp.Score > 5
ORDER BY 
    rp.CreationDate DESC
LIMIT 50;
