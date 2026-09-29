WITH RecursivePostHistory AS (
    SELECT 
        ph.Id,
        ph.PostId,
        ph.CreationDate,
        ph.UserId,
        ph.Comment,
        ph.PostHistoryTypeId,
        ROW_NUMBER() OVER (PARTITION BY ph.PostId ORDER BY ph.CreationDate DESC, ph.Id) AS rn
    FROM 
        PostHistory ph
),
UserReputation AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation,
        DENSE_RANK() OVER (ORDER BY u.Reputation DESC) AS ReputationRank
    FROM 
        Users u
),
PostsWithBadges AS (
    SELECT 
        p.Id AS PostId,
        COUNT(b.Id) AS BadgeCount,
        MAX(p.CreationDate) AS LatestActivityDate
    FROM 
        Posts p
    LEFT JOIN 
        Badges b ON p.OwnerUserId = b.UserId
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
    GROUP BY 
        p.Id
),
ClosedPosts AS (
    SELECT 
        p.Id AS PostId,
        ph.Comment AS CloseReason,
        ph.CreationDate AS CloseDate,
        ph.UserDisplayName AS ClosedBy
    FROM 
        Posts p
    JOIN 
        PostHistory ph ON p.Id = ph.PostId
    WHERE 
        ph.PostHistoryTypeId = 10  
),
RankedPosts AS (
    SELECT 
        p.Title,
        p.ViewCount,
        pw.BadgeCount,
        rp.UserId,
        ur.ReputationRank,
        cp.CloseReason,
        cp.CloseDate,
        cp.ClosedBy
    FROM 
        Posts p
    JOIN 
        PostsWithBadges pw ON p.Id = pw.PostId
    JOIN 
        RecursivePostHistory rp ON p.Id = rp.PostId AND rp.rn = 1  
    LEFT JOIN 
        ClosedPosts cp ON p.Id = cp.PostId
    LEFT JOIN 
        UserReputation ur ON rp.UserId = ur.UserId
)
SELECT 
    Title,
    ViewCount,
    BadgeCount,
    ReputationRank,
    CloseReason,
    ClosedBy,
    CASE 
        WHEN CloseDate IS NOT NULL THEN 'Closed'
        ELSE 'Active'
    END AS PostStatus
FROM 
    RankedPosts
WHERE 
    BadgeCount > 0 OR CloseReason IS NOT NULL
ORDER BY 
    ViewCount DESC, ReputationRank ASC
LIMIT 100;